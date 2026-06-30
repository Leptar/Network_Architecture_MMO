use std::sync::atomic::AtomicUsize;
use bevy::prelude::*;
use game_sockets::{GamePeer, protocols::QuicBackend, GameNetworkEvent, GameConnection, GameStream, GameStreamReliability};
use shared::*;
use bincode::*;
use uuid::uuid;

#[derive(Resource)]
struct ClientInfo {
    client_id: u32,
    is_registered: bool,
    pos_x: f32,
    pos_y: f32,
}

impl Default for ClientInfo {
    fn default() -> Self {
        Self {
            client_id: 0,
            is_registered: false,
            pos_x: 5050.0,
            pos_y: 5050.0,
        }
    }
}

#[derive(Resource)]
struct ClientSocket {
    peer: GamePeer,
    broker_connection: Option<GameConnection>,
    broker_stream: Option<GameStream>,
}

#[derive(Resource)]
struct InputBuffer([u8; 16]);

impl Default for InputBuffer {
    fn default() -> Self {
        Self([0; 16])
    }
}

#[derive(Resource)]
struct InputTimer(Timer);

impl Default for InputTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(0.1, TimerMode::Repeating))
    }
}


static INPUT_ID: AtomicUsize = AtomicUsize::new(0);

fn bind_socket(mut commands: Commands) {
    let peer = GamePeer::new(QuicBackend::new());
    peer.connect(BROK_IP, BROK_PORT).expect("Failed to connect to broker");

    commands.insert_resource(ClientSocket {
        peer,
        broker_connection: None,
        broker_stream: None,
    });

    println!("Connexion au broker en cours...");
}

fn receive_packet(
    mut socket: ResMut<ClientSocket>,
    mut client_info: ResMut<ClientInfo>,
) {
    while let Ok(Some(event)) = socket.peer.poll() {
        match event {
            GameNetworkEvent::Message { connection, stream, data } => {
                println!("Received message from connection id : {}, with stream id : {}, data : {:?}", connection.connection_id, stream.stream_id, data);

                let msg_tag: u8 = data[0];
                let msg_data = &data[1..];
                match msg_tag {
                    0x07 => {
                        if msg_data.len() < 4 { continue; }
                        let id = u32::from_le_bytes([msg_data[0], msg_data[1], msg_data[2], msg_data[3]]);
                        client_info.client_id = id;
                        println!("Mon client_id : {}", id);
                    },
                    0x04 => {
                        if msg_data.len() < 2 {
                            println!("Received invalid Broadcast message (too short)");
                            continue;
                        }
                        let payload_len = u16::from_le_bytes([msg_data[0], msg_data[1]]) as usize;
                        if msg_data.len() < 2 + payload_len {
                            println!("Received invalid Broadcast message (payload length mismatch)");
                            continue;
                        }
                        let payload = &msg_data[2..2 + payload_len];
                        println!("Received Broadcast message with payload length : {}, payload : {:?}", payload_len, payload);
                    }

                    0x06 => {
                        if msg_data.len() < 4 { continue; }
                        let id = u32::from_le_bytes([msg_data[0], msg_data[1], msg_data[2], msg_data[3]]);
                        client_info.client_id = id;
                        client_info.is_registered = true;
                        println!("Identité reçue ! Mon client_id : {}", id);
                    },

                    _ => {
                        println!("Received message with unknown tag : {}", msg_tag);
                    },
                }
            },

            GameNetworkEvent::Connected(connection) => {
                socket.broker_connection = Some(connection);
                println!("Connecté au broker (id : {})", connection.connection_id);

                match socket.peer.create_stream(connection, GameStreamReliability::Reliable) {
                    Ok(_) => println!("Creation du stream de communication avec le broker"),
                    Err(e) => println!("Erreur create_stream: {:?}", e),
                }
            },

            GameNetworkEvent::StreamCreated(connection, stream) => {
                socket.broker_stream = Some(stream.clone());
                println!("Stream créé avec le broker, stream id : {}", stream.stream_id);

                //envoir du ClientInit au broker
                let mut client_init: ClientInit = ClientInit{
                    client_id: 0, //le broker va remplir le client_id dans le ClientInit
                    pos_x: client_info.pos_x,
                    pos_y: client_info.pos_y
                };

                let serialized = bincode::serialize(&client_init).expect("Failed to serialize ClientInit");
                let mut data = vec![0x07];
                data.extend_from_slice(&serialized);

                if let (Some(connection), Some(stream)) = (&socket.broker_connection, &socket.broker_stream) {
                    let _ = socket.peer.send(connection, stream, bytes::Bytes::from(data));
                    println!("ClientInit envoyé au broker");
                }
                println!("Identification envoyée au broker");
            },

            GameNetworkEvent::Disconnected(connection) => {
                println!("Connection disconnected with id : {}", connection.connection_id);
            }
            _ => {}
        }
    }
}

fn send_input(
    mut socket: ResMut<ClientSocket>,
    client_id: Res<ClientInfo>,
    mut input_buffer: ResMut<InputBuffer>,
    mut timer: ResMut<InputTimer>,
    time: Res<Time>,
) {
    // Ne rien envoyer si on n'a pas encore reçu notre ID du Broker
    if !client_id.is_registered {
        return;
    }

    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }

    add_input_in_buffer(&mut input_buffer.0, INPUT_RIGHT);

    let input_packet: ClientInput = ClientInput {
        client_id: client_id.client_id,
        sequence_id: INPUT_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst) as u32,
        input: input_buffer.0,
    };

    let serialized = bincode::serialize(&input_packet).expect("Failed to serialize ClientInput");
    let mut data = vec![0x05];
    data.extend_from_slice(&serialized);

    if let (Some(connection), Some(stream)) = (&socket.broker_connection, &socket.broker_stream) {
        let _ = socket.peer.send(connection, stream, bytes::Bytes::from(data));
    }
}

fn add_input_in_buffer(buffer: &mut [u8; 16], input: u8) {
    for i in (1..buffer.len()).rev() {
        buffer[i] = buffer[i - 1];
    }
    buffer[0] = input;
}

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .init_resource::<ClientInfo>()
        .init_resource::<InputBuffer>()
        .init_resource::<InputTimer>()
        .add_systems(Startup, bind_socket)
        .add_systems(Update, (receive_packet, send_input).chain())
        .run();
}