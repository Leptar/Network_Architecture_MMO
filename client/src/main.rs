use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicU32;
use game_sockets::*;
use game_sockets::protocols::*;
use shared::*;

static CLIENT_ID: AtomicU32 = AtomicU32::new(0);
static BROKER_CONNECTION: Mutex<Option<GameConnection>> = Mutex::new(None);
static BROKER_STREAM: Mutex<Option<GameStream>> = Mutex::new(None);

static INPUT_BUFFER: Mutex<[u8; 16]> = Mutex::new([0; 16]);

fn receive_packet(mut peer: Arc<Mutex<GamePeer>>) {
    loop {
        while let Ok(Some(event)) = peer.lock().unwrap().poll() {
            match event {
                GameNetworkEvent::Message { connection, stream, data } => {
                    println!("Received message from connection id : {}, with stream id : {}, data : {:?}", connection.connection_id, stream.stream_id, data);

                    let msg_tag : u8 = data[0];
                    let msg_data = &data[1..];
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&String::from_utf8_lossy(&msg_data)) {
                        match msg_tag {
                            0x00 =>{//TODO: mettre le bon tag de msg pour avoir mon client ID
                                let client_id = CLIENT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                println!("Received client ID : {}, from connection id : {}, with stream id : {}", client_id, connection.connection_id, stream.stream_id);
                            },
                            0x04 => { //Format du Broadcast: payload_len: u16, payload: [u8]
                                if msg_data.len() < 2 {
                                    println!("Received invalid Broadcast message (too short) from connection id : {}, with stream id : {}", connection.connection_id, stream.stream_id);
                                    continue;
                                }
                                let payload_len = u16::from_be_bytes([msg_data[0], msg_data[1]]) as usize;
                                if msg_data.len() < 2 + payload_len {
                                    println!("Received invalid Broadcast message (payload length mismatch) from connection id : {}, with stream id : {}", connection.connection_id, stream.stream_id);
                                    continue;
                                }
                                let payload = &msg_data[2..2+payload_len];
                                println!("Received Broadcast message with payload length : {}, payload : {:?}, from connection id : {}, with stream id : {}", payload_len, payload, connection.connection_id, stream.stream_id);
                            }
                            _ => {
                                println!("Received message with unknown tag : {}, from connection id : {}, with stream id : {}", msg_tag, connection.connection_id, stream.stream_id);
                                continue;
                            },
                        }
                    }
                },
                GameNetworkEvent::Connected(connection) => {
                    BROKER_CONNECTION.lock().unwrap().replace(connection.clone());
                    println!("New connection established with id : {}", connection.connection_id);
                },
                GameNetworkEvent::Disconnected(connection) => {
                    println!("Connection disconnected with id : {}", connection.connection_id);
                }
                _ => {
                    println!("WARNING : Event received does not match any expected event : {:?}", event);
                },
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

fn send_input(mut peer: Arc<Mutex<GamePeer>>) {
    let client_id = CLIENT_ID.load(std::sync::atomic::Ordering::Relaxed);

    if BROKER_STREAM.lock().unwrap().is_none() {
        BROKER_STREAM.lock().unwrap().replace(GameStream::from(0));
    }


    loop {
        let connection = BROKER_CONNECTION.lock().unwrap().clone();
        let stream = BROKER_STREAM.lock().unwrap().clone();

        //ajoute une fausse input dans le buffer en ajoutant aléatoirement INPUT_...
        add_input_in_buffer(
            (if rand::random::<bool>() { INPUT_LEFT } else { 0 })
                | (if rand::random::<bool>() { INPUT_RIGHT } else { 0 })
                | (if rand::random::<bool>() { INPUT_UP } else { 0 })
                | (if rand::random::<bool>() { INPUT_DOWN } else { 0 })
        );

        let msg = ClientInput {
            client_id,
            input: INPUT_BUFFER.lock().unwrap().clone(),
        };

        let msg_data = serde_json::to_vec(&msg).unwrap();
        let mut data = vec![0x05];
        data.extend_from_slice(&msg_data);

        if let (Some(connection), Some(stream)) = (connection, stream) {
            peer.lock().unwrap().send(&connection, &stream, bytes::Bytes::from(data));
        }

        let rand_time = rand::random::<u64>() % 100 + 50; // Envoie des données d'entrée toutes les 50 à 150ms
        std::thread::sleep(std::time::Duration::from_millis(rand_time));
    }
}

fn add_input_in_buffer(input: u8) {
    let mut buffer = INPUT_BUFFER.lock().unwrap();
    for i in (1..buffer.len()).rev() {
        buffer[i] = buffer[i - 1];
    }
    
    buffer[0] = input;
}

#[tokio::main]
async fn main() {
    let broker_addr = "127.0.0.1";
    let broker_port = 9000;

    let peer = Arc::new(std::sync::Mutex::new(GamePeer::new(UdpBackend::new())));
    let peer2 = peer.clone();

    peer.lock().unwrap().connect(broker_addr, broker_port).expect("Failed to connect to broker");

    std::thread::spawn(move || receive_packet(peer));
    std::thread::spawn(move || send_input(peer2));

    tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
    println!("Shutting down client...");
}