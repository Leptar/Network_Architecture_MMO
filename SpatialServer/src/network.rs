use bevy::prelude::*;
use game_sockets::*;
use crate::components::{ClientId, CurrentShard, NearbyShards, PlayerEntities, Position, SpatialSocket};
use bytes::Bytes;
use game_sockets::protocols::{QuicBackend};
use crate::messages::*;
use shared::{BROK_IP, BROK_PORT, PAYLOAD_BOOT_SHARD, PAYLOAD_CROSSING_ALERT, TAG_ADMIN_CONNECT, TAG_ADMIN_ROUTE_SEND};
use crate::quadtree::QuadTree;

pub fn connect_to_broker(mut commands: Commands) {

    let peer = GamePeer::new(QuicBackend::new());


    println!("Tentative de connexion au Broker sur {}...", BROK_IP);
    // Tenter la connexion
    match peer.connect(BROK_IP, BROK_PORT) {
        Ok(_) => {
            println!("Demande de connexion envoyée au Broker. En attente de l'établissement de la connexion...");
        }
        Err(e) => {
            panic!("Échec : Impossible de se connecter au Broker. Erreur : {:?}", e);
        }
    }

    // On insère notre ressource globale dans l'ECS
    commands.insert_resource(SpatialSocket {
        peer,
        broker_conn: None
    });
}

pub fn receive_position_updates(
    mut commands: Commands,
    mut socket: ResMut<SpatialSocket>,
    mut player_entities: ResMut<PlayerEntities>,
    mut query: Query<(Entity, &ClientId, &mut Position)>,
    quad_tree: Res<QuadTree>,
    mut boot_evts: MessageWriter<BootShardMessage>,
) {
    while let Ok(Some(event)) = socket.peer.poll() {
        match event {
            GameNetworkEvent::Connected(conn) => {
                println!("Connexion établie avec le Broker ");
                socket.broker_conn = Some(conn);

                //Ouverture du stream
                match socket.peer.create_stream(conn, GameStreamReliability::Reliable) {
                    Ok(_) => println!("Demande de création du stream de communication avec le Broker envoyée"),
                    Err(e) => println!("Erreur lors de la création du stream : {:?}", e),
                }
            }

            GameNetworkEvent::StreamCreated(connection, stream) => {
                println!("Stream créé avec le Broker (id : {}, stream id : {})", connection.connection_id, stream.stream_id);

                let mut auth_packet = Vec::new();
                auth_packet.push(TAG_ADMIN_CONNECT); // Tag 0x0A

                // On écrit notre nom "spatial" sur 32 octets pour le Broker
                let mut name_bytes = [0u8; 32];
                let name_str = b"spatial";
                name_bytes[..name_str.len()].copy_from_slice(name_str);
                auth_packet.extend_from_slice(&name_bytes);

                let stream = GameStream::from(0u16); // Stream fiable
                let _ = socket.peer.send(&connection, &stream, Bytes::from(auth_packet));

                println!("Carte d'identité envoyée : Je suis le 'spatial'");

                for (shard_id, bounds) in quad_tree.get_leaves() {
                    println!("Demande de Boot pour Shard {} -> Zone: {:?}", shard_id, bounds);
                    boot_evts.write(BootShardMessage { shard_id, bounds });
                }
            }

            GameNetworkEvent::Message { data, .. } => {
                if data.is_empty() { continue; }

                let tag = data[0];
                let rest = &data[1..];

                if tag == 0x10 {
                    if rest.len() < 12 { continue; }

                    let client_id = u32::from_le_bytes([rest[0], rest[1], rest[2], rest[3]]);
                    let x = f32::from_le_bytes([rest[4], rest[5], rest[6], rest[7]]);
                    let y = f32::from_le_bytes([rest[8], rest[9], rest[10], rest[11]]);
                    let nouvelle_position = Vec2::new(x, y);

                    let joueur_trouve = false;

                    if let Some(&entity) = player_entities.map.get(&client_id) {
                        if let Ok(mut pos) = query.get_mut(entity) {
                            pos.2.0 = nouvelle_position;
                        }
                    }

                    if !joueur_trouve {
                        println!("Nouveau joueur {} détecté via le réseau en {}, {}", client_id, x, y);
                        let new_player = commands.spawn((
                            ClientId(client_id),
                            Position(nouvelle_position),
                            CurrentShard(None),        // Il n'a pas encore de shard assigné
                            NearbyShards(Vec::new()),  // Mémoire radar vide au départ
                        )).id();
                        player_entities.map.insert(client_id, new_player);
                    }
                }
            }
            _ => {}
        }
    }
}

pub(crate) fn flush_network_messages(
    mut sub_reader: MessageReader<SubscribeMessage>,
    mut unsub_reader: MessageReader<UnsubscribeMessage>,
    mut alert_reader: MessageReader<CrossingAlertMessage>,
    mut boot_reader: MessageReader<BootShardMessage>,
    socket: ResMut<SpatialSocket>,
) {
    // Par défaut, stream 0 pour les messages fiables
    let stream = GameStream::from(0u16);
    if let Some(conn) = &socket.broker_conn {
        // Tag 0x01
        for sub in sub_reader.read() {
            let mut buffer = Vec::new();
            buffer.push(0x01); // Tag
            buffer.extend_from_slice(&sub.client_id.to_le_bytes()); // Client ID

            // Topic (32 bytes max). On formate en "shard:X"
            let mut topic_bytes = [0u8; 32];
            let topic_str = format!("shard:{}", sub.topic);
            let bytes = topic_str.as_bytes();
            let len = bytes.len().min(32);
            topic_bytes[..len].copy_from_slice(&bytes[..len]);
            buffer.extend_from_slice(&topic_bytes);

            // Envoi via la socket
            let len = buffer.len();
            let _ = socket.peer.send(conn, &stream, Bytes::from(buffer));
            println!("[RÉSEAU] Envoi Subscribe ({} octets) pour le client {}", len, sub.client_id);
        }

        // Tag 0x02
        for unsub in unsub_reader.read() {
            let mut buffer = Vec::new();
            buffer.push(0x02); // Tag
            buffer.extend_from_slice(&unsub.client_id.to_le_bytes());

            let mut topic_bytes = [0u8; 32];
            let topic_str = format!("shard:{}", unsub.topic);
            let bytes = topic_str.as_bytes();
            let len = bytes.len().min(32);
            topic_bytes[..len].copy_from_slice(&bytes[..len]);
            buffer.extend_from_slice(&topic_bytes);

            // Envoi via la socket
            let len = buffer.len();
            let _ = socket.peer.send(conn, &stream, Bytes::from(buffer));
            println!("[RÉSEAU] Envoi Unsubscribe ({} octets) pour le client {}", len, unsub.client_id);
        }

        // Tag 0x99 (Crossing)
        for alert in alert_reader.read() {
            let mut buffer = Vec::new();
            buffer.push(TAG_ADMIN_ROUTE_SEND); // Tag Publish

            // target : dgs source
            let mut target_bytes = [0u8; 32];
            let target_str = format!("shard:{}", alert.source_shard);
            let bytes = target_str.as_bytes();
            let len = bytes.len().min(32);
            target_bytes[..len].copy_from_slice(&bytes[..len]);
            buffer.extend_from_slice(&target_bytes);

            // Construction du Payload interne (Custom)
            let mut payload = Vec::new();
            payload.push(PAYLOAD_CROSSING_ALERT); // Tag CrossingAlert
            payload.extend_from_slice(&alert.client_id.to_le_bytes());
            payload.push(alert.involved_shards.len() as u8);
            for &shard_id in &alert.involved_shards {
                payload.extend_from_slice(&shard_id.to_le_bytes());
            }

            // Ajout de la taille du payload (u16 Little Endian)
            let payload_len = payload.len() as u16;
            buffer.extend_from_slice(&payload_len.to_le_bytes());
            buffer.extend_from_slice(&payload);

            // Envoi via la socket
            let len = buffer.len();
            let _ = socket.peer.send(conn, &stream, Bytes::from(buffer));
            println!("[RÉSEAU] Envoi CrossingAlert vers {} ({} octets)", target_str, len);
        }

        for boot in boot_reader.read() {
            let mut buffer = Vec::new();
            buffer.push(TAG_ADMIN_ROUTE_SEND);

            // Topic ciblé : Le salon d'administration de l'Orchestrateur
            let mut target_bytes = [0u8; 32];
            let target_str = b"orchestrator";
            target_bytes[..target_str.len()].copy_from_slice(target_str);
            buffer.extend_from_slice(&target_bytes);

            // Construction du Payload interne
            let mut payload = Vec::new();
            payload.push(PAYLOAD_BOOT_SHARD);

            // L'ID du shard à démarrer
            payload.extend_from_slice(&boot.shard_id.to_le_bytes());

            // Les coordonnées de la zone
            payload.extend_from_slice(&boot.bounds.min.x.to_le_bytes());
            payload.extend_from_slice(&boot.bounds.min.y.to_le_bytes());
            payload.extend_from_slice(&boot.bounds.max.x.to_le_bytes());
            payload.extend_from_slice(&boot.bounds.max.y.to_le_bytes());

            let payload_len = payload.len() as u16;
            buffer.extend_from_slice(&payload_len.to_le_bytes());
            buffer.extend_from_slice(&payload);

            // Envoi via la socket
            let len = buffer.len();
            let _ = socket.peer.send(conn, &stream, Bytes::from(buffer));
            println!("[RÉSEAU] Envoi Ordre de Boot (0x90) à l'Orchestrateur pour Shard {} ({} octets)", boot.shard_id, len);
        }
    }

}