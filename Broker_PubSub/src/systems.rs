use bevy::prelude::*;
use game_sockets::{GamePeer, protocols::QuicBackend, GameNetworkEvent};
use crate::resources::{BrokerSocket, ClientRegistry, SubscriptionMap, ClientShardMap};

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
    mut shard_map: ResMut<ClientShardMap>
) {
    while let Ok(Some(event)) = socket.peer.poll() {
        match event {
            GameNetworkEvent::Message { connection, stream, data } => {
                if data.is_empty() { continue; }

                let tag = data[0];
                let rest = &data[1..];

                match tag {
                    0x00 => {
                        if rest.len() < 32 { continue; }

                        let topic = String::from_utf8_lossy(&rest[0..32])
                            .trim_matches('\0')
                            .to_string();

                        shard_map.shard_connections.insert(topic.clone(), connection);

                        println!("Shard '{}' enregistré", topic);
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

                        // Mettre à jour la map inversée
                        shard_map.map.insert(client_id, topic.clone());

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

                        // Retirer de la map inversée
                        shard_map.map.remove(&client_id);

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
                        broadcast.push(0x04u8);
                        broadcast.extend_from_slice(&(payload_len as u16).to_le_bytes());
                        broadcast.extend_from_slice(payload);
                        let broadcast = bytes::Bytes::from(broadcast);

                        // Envoyer à tous les abonnés
                        if let Some(subscribers) = subs.subscriptions.get(&topic) {
                            for &client_id in subscribers {
                                if let Some(conn) = clients.clients.get(&client_id) {
                                    let _ = socket.peer.send(
                                        conn,
                                        &stream,
                                        broadcast.clone()
                                    );
                                }
                            }
                        }
                    }
                    0x05 => {
                        if rest.len() < 20 { continue; } // 4 (client_id) + 16 (input)

                        let client_id = u32::from_le_bytes([rest[0], rest[1], rest[2], rest[3]]);
                        let input = &rest[4..20];

                        if let Some(topic) = shard_map.map.get(&client_id) {
                            if let Some(shard_conn) = shard_map.shard_connections.get(topic) {

                                // Reconstruire le message pour le shard
                                let mut msg = Vec::new();
                                msg.push(0x05u8);
                                msg.extend_from_slice(&client_id.to_le_bytes());
                                msg.extend_from_slice(input);

                                let stream = game_sockets::GameStream::from(0u16);
                                let _ = socket.peer.send(
                                    shard_conn,
                                    &stream,
                                    bytes::Bytes::from(msg)
                                );

                                println!("Input client {} relayé au shard '{}'", client_id, topic);
                            }
                        }
                    }
                    0x07 => {
                        // Le client se déclare
                        let client_id = clients.next_id;
                        clients.next_id += 1;
                        clients.clients.insert(client_id, connection);

                        let stream_resp = game_sockets::GameStream::from(0u16);
                        let mut response = Vec::new();
                        response.push(0x06u8);
                        response.extend_from_slice(&client_id.to_le_bytes());

                        let _ = socket.peer.send(
                            &connection,
                            &stream_resp,
                            bytes::Bytes::from(response)
                        );

                        println!("Client identifié, id assigné : {}", client_id);
                    }
                    _ => { println!("Tag inconnu : {:#x}", tag); }
                }
            }
            GameNetworkEvent::Connected(conn) => {
                let stream = game_sockets::GameStream::from(0u16);
                let client_id = clients.next_id;
                clients.next_id += 1;
                clients.clients.insert(client_id, conn);

                let mut response = Vec::new();
                response.push(0x06u8);
                response.extend_from_slice(&client_id.to_le_bytes());

                let _ = socket.peer.send(
                    &conn,
                    &stream,
                    bytes::Bytes::from(response)
                );

                println!("Nouveau client connecté, id assigné : {}", client_id);
            }
            _ => {}
        }
    }
}