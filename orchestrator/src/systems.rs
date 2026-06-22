use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};
use bevy::prelude::*;
use shared::*;
use game_sockets::*;
use game_sockets::protocols::QuicBackend;
use uuid::Uuid;

use crate::resources;
use crate::resources::*;

const HOT_SERVERS_MIN: usize = 4;

static DGS_ID: AtomicU32 = AtomicU32::new(0);

fn debug_msg(msg: String) {
    println!("ORCHESTRATOR : {}", msg);
}

//------------------------ StartUp Function ------------------------//

pub fn startup_orchestrator(mut commands: Commands) {
    commands.insert_resource(bind_socket());
    start_inital_server();
}

fn bind_socket() -> GameSocket {
    let peer = GamePeer::new(QuicBackend::new());
    peer.listen(ORCH_IP, ORCH_PORT).unwrap();
    debug_msg(format!("Game socket initialized and listening on {}:{}", ORCH_IP, ORCH_PORT));

    match peer.connect(BROK_IP, BROK_PORT) {
        Ok(_) => debug_msg(format!("Successfully send a connection request to Broker at {}:{}", BROK_IP, BROK_PORT)),
        Err(e) => debug_msg(format!("Failed to connect to Broker at {}:{} - Error: {}", BROK_IP, BROK_PORT, e)),
    }

    let new_hashmap: std::collections::HashMap<Uuid, resources::DGSNetworkInfo> = std::collections::HashMap::new();

    GameSocket {
        peer,
        connection_broker: None,
        stream_broker: None,
        dgs_network_info_dictionary: new_hashmap,
    }
}

fn start_inital_server() {
    for _ in 0..HOT_SERVERS_MIN {
        start_servers();
    }
}

//------------------------ Update Functions ------------------------//

pub fn receive_packets(
    mut socket: ResMut<GameSocket>,
    mut redis_connection: ResMut<RedisConnection>,
) {
    while let Ok(Some(event)) = socket.peer.poll() {
        match event {
            GameNetworkEvent::Message { connection, stream, data } => {
                if data.is_empty() {
                    debug_msg(format!("Received empty message from connection id : {}", connection.connection_id));
                    continue;
                }

                // Message du Broker
                let is_broker = socket.connection_broker
                    .as_ref()
                    .map(|b| b.connection_id == connection.connection_id)
                    .unwrap_or(false);

                if is_broker {
                    debug_msg(format!("Received message from Broker, stream id : {}", stream.stream_id));

                    if data[0] == TAG_ADMIN_ROUTE_RECEIVE {
                        let payload = &data[1..];
                        if !payload.is_empty() {
                            let internal_tag = payload[0];

                            if internal_tag == PAYLOAD_BOOT_SHARD && payload.len() >= 21 {
                                let shard_id = u32::from_le_bytes([payload[1], payload[2], payload[3], payload[4]]);
                                let min_x = f32::from_le_bytes([payload[5], payload[6], payload[7], payload[8]]);
                                let min_y = f32::from_le_bytes([payload[9], payload[10], payload[11], payload[12]]);
                                let max_x = f32::from_le_bytes([payload[13], payload[14], payload[15], payload[16]]);
                                let max_y = f32::from_le_bytes([payload[17], payload[18], payload[19], payload[20]]);

                                debug_msg(format!("Demande de Boot pour le Shard {} ! Zone : [Min:({}, {}), Max:({}, {})]", shard_id, min_x, min_y, max_x, max_y));

                                let mut con = redis_connection.client.get_connection().expect("Failed to connect to Redis");

                                if let Some(idle_uuid) = get_idle_server_id(&mut con) {
                                    let _: () = redis::cmd("HSET")
                                        .arg(format!("server:{}", idle_uuid))
                                        .arg("state").arg("warmup")
                                        .query(&mut con).expect("Failed to lock server in Redis");

                                    let mut buffer = Vec::new();
                                    buffer.push(TAG_ADMIN_ROUTE_SEND);

                                    let mut target_bytes = [0u8; 32];
                                    let target_str = format!("dgs_{}", idle_uuid);
                                    let bytes = target_str.as_bytes();
                                    let len = bytes.len().min(32);
                                    target_bytes[..len].copy_from_slice(&bytes[..len]);
                                    buffer.extend_from_slice(&target_bytes);

                                    let mut wakeup_payload = Vec::new();
                                    wakeup_payload.push(PAYLOAD_WAKEUP_COMMAND);
                                    wakeup_payload.extend_from_slice(&shard_id.to_le_bytes());
                                    wakeup_payload.extend_from_slice(&min_x.to_le_bytes());
                                    wakeup_payload.extend_from_slice(&min_y.to_le_bytes());
                                    wakeup_payload.extend_from_slice(&max_x.to_le_bytes());
                                    wakeup_payload.extend_from_slice(&max_y.to_le_bytes());

                                    let payload_len = wakeup_payload.len() as u16;
                                    buffer.extend_from_slice(&payload_len.to_le_bytes());
                                    buffer.extend_from_slice(&wakeup_payload);

                                    if let (Some(broker_conn), Some(broker_stream)) = (&socket.connection_broker, &socket.stream_broker) {
                                        let _ = socket.peer.send(broker_conn, broker_stream, bytes::Bytes::from(buffer));
                                        debug_msg(format!("WakeUp envoyé au serveur : {}", target_str));
                                    }
                                } else {
                                    debug_msg("Aucun serveur DGS inactif disponible.".to_string());
                                }
                            }
                        }
                    }
                    continue;
                }

                // Pas le broker → c'est un DGS
                let broker_not_connected = socket.connection_broker.is_none();
                if broker_not_connected {
                    debug_msg(format!("WARNING : Broker not connected yet, message from {}", connection.connection_id));
                    continue;
                }

                // Message des DGS
                let dgs_info_opt = socket.dgs_network_info_dictionary.get(&connection.connection_id).cloned();

                if let Some(mut dgs_info) = dgs_info_opt {
                    let updated_id = resolve_dgs_message(&mut dgs_info, &data, &redis_connection.client, &socket);
                    if updated_id {
                        socket.dgs_network_info_dictionary.insert(connection.connection_id, dgs_info);
                    }
                } else {
                    let mut new_dgs_info = DGSNetworkInfo {
                        dgs_id: u32::MAX,
                        connection_dgs: Some(connection),
                        stream_dgs: Some(stream.clone()),
                    };
                    debug_msg(format!("New DGS connection, connection id : {}", connection.connection_id));
                    let updated_id = resolve_dgs_message(&mut new_dgs_info, &data, &redis_connection.client, &socket);
                    socket.dgs_network_info_dictionary.insert(connection.connection_id, new_dgs_info);
                }
            }

            GameNetworkEvent::Connected(connection) => {
                if socket.connection_broker.is_none() {
                    debug_msg(format!("Connected to Broker with connection id : {}", connection.connection_id));
                    socket.connection_broker = Some(connection);
                    socket.peer.create_stream(connection, GameStreamReliability::Unreliable).unwrap();
                } else {
                    debug_msg(format!("New DGS connected with connection id : {}", connection.connection_id));
                }
            }

            GameNetworkEvent::StreamCreated(connection, stream) => {
                if let Some(broker_conn) = socket.connection_broker.clone() {
                    if connection.connection_id == broker_conn.connection_id {
                        socket.stream_broker = Some(stream.clone());
                        debug_msg(format!("Stream created with Broker, stream id : {}", stream.stream_id));

                        let mut auth_packet = Vec::new();
                        auth_packet.push(TAG_ADMIN_CONNECT);
                        let mut name_bytes = [0u8; 32];
                        let name_str = b"orchestrator";
                        name_bytes[..name_str.len()].copy_from_slice(name_str);
                        auth_packet.extend_from_slice(&name_bytes);

                        let _ = socket.peer.send(&broker_conn, &stream, bytes::Bytes::from(auth_packet));
                        debug_msg("Identification envoyée au Broker : 'orchestrator'".to_string());
                    }
                }
            }

            _ => {
                debug_msg(format!("WARNING : Unexpected event : {:?}", event));
            }
        }
    }
}

// Retourne true si le dgs_id a été mis à jour
fn resolve_dgs_message(connection_dgs_info: &mut DGSNetworkInfo, data: &[u8], client_redis: &redis::Client, socket: &GameSocket) -> bool {
    if data.is_empty() { return false; }

    match data[0] {
        0x20 | 0x21 | 0x22 | 0x23 | 0x24 => {
            relay_to_target_dgs(data, socket);
            false
        }
        _ => {
            heartbeat_deserialize(client_redis, data, connection_dgs_info)
        }
    }
}

fn relay_to_target_dgs(data: &[u8], socket: &GameSocket) {
    if data.len() < 5 { return; }

    let target_shard_id = u32::from_le_bytes([data[1], data[2], data[3], data[4]]);

    let payload = &data[5..];
    let mut forwarded = Vec::with_capacity(1 + payload.len());
    forwarded.push(data[0]);
    forwarded.extend_from_slice(payload);

    let target = socket.dgs_network_info_dictionary.values()
        .find(|dgs| dgs.dgs_id == target_shard_id);

    if let Some(target) = target {
        if let (Some(conn), Some(stream)) = (&target.connection_dgs, &target.stream_dgs) {
            match socket.peer.send(conn, stream, bytes::Bytes::from(forwarded)) {
                Ok(_) => debug_msg(format!("Message relayé au DGS {}", target_shard_id)),
                Err(e) => debug_msg(format!("Erreur relay vers DGS {}: {:?}", target_shard_id, e)),
            }
        }
    } else {
        debug_msg(format!("WARNING : DGS cible {} non trouvé", target_shard_id));
    }
}

fn start_servers() {
    let id = DGS_ID.fetch_add(1, Ordering::Relaxed);
    let nom_conteneur = format!("dgs-{}", id);

    Command::new("docker")
        .args(&["rm", "-f", &nom_conteneur])
        .output()
        .ok();

    Command::new("docker")
        .args(&[
            "run",
            "--name", &nom_conteneur,
            "-e", &format!("ORCHESTRATOR_IP=host.docker.internal"),
            "-e", &format!("ORCHESTRATOR_PORT={}", ORCH_PORT),
            "-e", &format!("BROKER_IP=host.docker.internal"),
            "-e", &format!("BROKER_PORT={}", BROK_PORT),
            "-e", &format!("SERVER_ID={}", id),
            "--add-host=host.docker.internal:host-gateway",
            "dgs-image",
        ])
        .spawn()
        .expect("Failed to start game server");
}

fn stop_all_servers() {
    let current_id = DGS_ID.load(Ordering::Relaxed);
    for i in 0..current_id {
        let nom_conteneur = format!("dgs-{}", i);
        Command::new("docker").args(&["stop", &nom_conteneur]).output().ok();
        Command::new("docker").args(&["rm", &nom_conteneur]).output().ok();
        debug_msg(format!("Stopped and removed container {}", nom_conteneur));
    }
}

// Retourne true si dgs_id a été mis à jour
fn heartbeat_deserialize(client_redis: &redis::Client, data: &[u8], connection_dgs_info: &mut DGSNetworkInfo) -> bool {
    if data.is_empty() {
        debug_msg("Received empty heartbeat message, ignoring...".to_string());
        return false;
    }

    let mut con = client_redis.get_connection().expect("Failed to connect to Redis");
    let msg = String::from_utf8_lossy(data);

    if let Ok(heartbeat) = serde_json::from_str::<Heartbeat>(&msg) {
        debug_msg(format!("Receive heartbeat from DGS {}", heartbeat.id));

        let mut updated = false;
        if connection_dgs_info.dgs_id == u32::MAX {
            connection_dgs_info.dgs_id = heartbeat.id;
            updated = true;
        }

        let _: () = redis::cmd("HSET")
            .arg(format!("server:{}", heartbeat.id))
            .arg("port").arg(heartbeat.port)
            .arg("ip").arg(&heartbeat.ip)
            .arg("zone").arg(&heartbeat.zone)
            .arg("player_count").arg(heartbeat.player_count)
            .arg("max_players").arg(heartbeat.max_players)
            .arg("status").arg(if heartbeat.status == shared::ServerStatus::Available { "available" } else { "full" })
            .arg("state").arg(match heartbeat.state {
            shared::ServerState::WarmUp => "warmup",
            shared::ServerState::Idle => "idle",
            shared::ServerState::Running => "running",
            shared::ServerState::Stopping => "stopping",
        })
            .query(&mut con).expect("Failed to update Redis");

        let _: () = redis::cmd("EXPIRE")
            .arg(format!("server:{}", heartbeat.id))
            .arg(15)
            .query(&mut con).expect("Failed to set TTL");

        updated
    } else {
        debug_msg("Failed to parse heartbeat, ignoring...".to_string());
        false
    }
}

fn count_idle_servers(con: &mut redis::Connection) -> usize {
    let keys: Vec<String> = redis::cmd("KEYS")
        .arg("server:*")
        .query(con)
        .expect("Failed to query Redis for server keys");

    keys.iter().filter(|key| {
        let state: String = redis::cmd("HGET")
            .arg(*key)
            .arg("state")
            .query(con)
            .unwrap_or_default();
        state == "idle"
    }).count()
}

fn count_servers(con: &mut redis::Connection) -> usize {
    let keys: Vec<String> = redis::cmd("KEYS")
        .arg("server:*")
        .query(con)
        .expect("Failed to query Redis for server keys");
    keys.len()
}

pub fn scaler_loop(
    client: Res<RedisConnection>,
    mut timer: ResMut<ScalerLoopTimer>,
    time: Res<Time>,
) {
    timer.timer.tick(time.delta());
    if !timer.timer.just_finished() { return; }

    let mut con = client.client.get_connection().expect("Failed to connect to Redis");
    let idle_servers = count_idle_servers(&mut con);

    for _ in idle_servers..HOT_SERVERS_MIN {
        start_servers();
    }

    debug_msg(format!("Scaler loop: {} idle servers, {} total servers", idle_servers, count_servers(&mut con)));
}

fn get_idle_server_id(con: &mut redis::Connection) -> Option<String> {
    let keys: Vec<String> = redis::cmd("KEYS")
        .arg("server:*")
        .query(con)
        .unwrap_or_default();

    for key in keys {
        let state: String = redis::cmd("HGET")
            .arg(&key)
            .arg("state")
            .query(con)
            .unwrap_or_default();

        if state == "idle" {
            return Some(key.replace("server:", ""));
        }
    }
    None
}