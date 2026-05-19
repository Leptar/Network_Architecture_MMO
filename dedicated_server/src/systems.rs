use bevy::prelude::*;
use game_sockets::{GamePeer, protocols::UdpBackend, GameNetworkEvent};
use crate::resources::{ServerConfig, GameSocket, PlayerRegistry, HeartbeatTimer, OrchestratorConnection};

pub fn bind_socket(mut commands: Commands, config: Res<ServerConfig>) {
    let peer = GamePeer::new(UdpBackend::new());
    peer.listen("0.0.0.0", config.port).unwrap();

    // Se connecter à l'orchestrateur
    let parts: Vec<&str> = config.orchestrator_addr.split(':').collect();
    let orch_ip = parts[0];
    let orch_port: u16 = parts[1].parse().unwrap();
    peer.connect(orch_ip, orch_port).unwrap();

    commands.insert_resource(GameSocket { peer });
    commands.insert_resource(OrchestratorConnection { connection: None });
    println!("Serveur démarré sur le port {}", config.port);
}

pub fn receive_packets(
    mut socket: ResMut<GameSocket>,
    mut registry: ResMut<PlayerRegistry>,
    mut orch_conn: ResMut<OrchestratorConnection>,
) {
    while let Ok(Some(event)) = socket.peer.poll() {
        match event {
            GameNetworkEvent::Message { connection, stream, data } => {
                let msg = String::from_utf8_lossy(&data);
                println!("Message reçu : {}", msg);

                // Parser le JSON pour extraire le username
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&msg) {
                    if let Some(username) = json["username"].as_str() {

                        // Générer un ID unique pour ce joueur
                        let player_id = uuid::Uuid::new_v4().to_string();

                        // Enregistrer le joueur
                        registry.players.insert(connection, username.to_string());
                        println!("Joueur {} connecté avec id {}", username, player_id);

                        // Répondre WELCOME
                        let response = serde_json::json!({
                "player_id": player_id
            }).to_string();

                        let bytes = bytes::Bytes::from(response.into_bytes());
                        let _ = socket.peer.send(&connection, &stream, bytes);
                    }
                }
            }
            GameNetworkEvent::Connected(conn) => {
                println!("Connecté à : {:?}", conn);
                orch_conn.connection = Some(conn);
            }
            _ => {}
        }
    }
}

pub fn send_heartbeat(
    mut socket: ResMut<GameSocket>,
    config: Res<ServerConfig>,
    registry: Res<PlayerRegistry>,
    mut timer: ResMut<HeartbeatTimer>,
    time: Res<Time>,
    orch_conn: Res<OrchestratorConnection>,
) {
    // Avance le timer
    timer.0.tick(time.delta());

    if !timer.0.just_finished() {
        return;
    }

    // Construire le heartbeat
    let heartbeat = shared::Heartbeat {
        id: config.id.clone(),
        ip: "127.0.0.1".to_string(),
        port: config.port,
        zone: config.zone.clone(),
        player_count: registry.players.len(),
        max_players: config.max_players,
    };

    let json = serde_json::to_string(&heartbeat).unwrap();
    println!("Envoi heartbeat : {}", json);

    if let Some(conn) = &orch_conn.connection {
        let stream = game_sockets::GameStream::from(0u16);
        let bytes = bytes::Bytes::from(json.into_bytes());
        let _ = socket.peer.send(conn, &stream, bytes);
        println!("Heartbeat envoyé à l'orchestrateur !");
    }
}