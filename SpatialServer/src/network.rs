use bevy::prelude::*;
use game_sockets::*;
use crate::components::{ClientId, Position, SpatialSocket};
use bytes::Bytes;
use crate::messages::*;


pub fn receive_position_updates(
    mut commands: Commands,
    // mut socket: ResMut<SpatialSocket>,
    // On a besoin d'une requête pour trouver l'entité correspondant à un ClientId
    mut query: Query<(Entity, &ClientId, &mut Position)>
) {
    /* // BOUCLE DE LECTURE RÉSEAU (Commentée en attendant ta socket)
    while let Ok(Some(event)) = socket.peer.poll() {
        if let GameNetworkEvent::Message { data, .. } = event {
            if data.is_empty() { continue; }

            let tag = data[0];
            let rest = &data[1..];

            if tag == 0x10 { // PositionUpdate
                // 1. Extraire les données binaires
                // ...

                // 2. Mettre à jour l'ECS
                // ...
            }
        }
    }
    */
}

pub(crate) fn flush_network_messages(
    mut sub_reader: MessageReader<SubscribeMessage>,
    mut unsub_reader: MessageReader<UnsubscribeMessage>,
    mut alert_reader: MessageReader<CrossingAlertMessage>,
    mut socket: ResMut<SpatialSocket>, // Ta ressource de connexion au Broker
) {
    // Par défaut, stream 0 pour les messages fiables
    let stream = GameStream::from(0u16);

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
        let _ = socket.peer.send(&socket.broker_conn, &stream, Bytes::from(buffer.clone()));
        println!("[RÉSEAU] Envoi Subscribe ({} octets) pour le client {}", buffer.len(), sub.client_id);
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
        let _ = socket.peer.send(&socket.broker_conn, &stream, Bytes::from(buffer.clone()));
        println!("[RÉSEAU] Envoi Unsubscribe ({} octets) pour le client {}", buffer.len(), unsub.client_id);
    }

    // Tag 0x03
    for alert in alert_reader.read() {
        let mut buffer = Vec::new();
        buffer.push(0x03); // Tag Publish

        // Topic ciblé : l'ancien shard (le Shard Source)
        let mut topic_bytes = [0u8; 32];
        let topic_str = format!("shard:{}", alert.source_shard);
        let bytes = topic_str.as_bytes();
        let len = bytes.len().min(32);
        topic_bytes[..len].copy_from_slice(&bytes[..len]);
        buffer.extend_from_slice(&topic_bytes);

        // Construction du Payload interne (Custom)
        let mut payload = Vec::new();
        payload.push(0x99); // Tag CrossingAlert (arbitraire, A valider avec vous)
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
        let _ = socket.peer.send(&socket.broker_conn, &stream, Bytes::from(buffer.clone()));
        println!("[RÉSEAU] Envoi CrossingAlert vers {}: {:?}", topic_str, buffer);
    }
}