use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::net::UdpSocket;
use shared::Heartbeat;

//Variable to manage game server
const HOT_SERVERS_MIN: usize = 2;
const CHECK_TIME_SERVERS_AVAILABLE: usize = 2;
const ORCH_PORT: usize = 22555;
const UPDATE_REDIS_TIME: usize = 5;

static DGS_ID: AtomicUsize = AtomicUsize::new(0);

async fn start_servers()
{
    for _ in 0..HOT_SERVERS_MIN {
        let id = DGS_ID.fetch_add(1, Ordering::Relaxed);
        let nom_conteneur = format!("dgs-{}", id);
        let orch_addr = format!("ORCH_ADDR=host.docker.internal:{}", ORCH_PORT);

        Command::new("docker")
            .args(&[
                "run",
                "--name",
                &nom_conteneur,
                "-e",
                &orch_addr,
                "--rm",
                "dgs-image",
            ])
            .spawn()
            .expect("Failed to start game server 1");
    }
}

fn stop_servers()
{

}


async fn heartbeat_listener (){
    //format : HEARTBEAT { id, ip, port, zone, player_count }
    let socket = UdpSocket::bind(("0.0.0.0", ORCH_PORT as u16)).await.expect("Failed to start heartbeat listener");

    loop {
        let mut buf = [0; 1024];
        let (len, addr) = socket.recv_from(&mut buf).await.expect("Failed to receive heartbeat");
        let msg = String::from_utf8_lossy(&buf[..len]);
        println!("Received heartbeat from {}: {}", addr, msg);

        if let Ok(heartbeat) = serde_json::from_str::<Heartbeat>(&msg) {
            println!("Parsed heartbeat: {:?}", heartbeat);
        } else {
            println!("Failed to parse heartbeat, ignoring...");
        }
    }
}

#[tokio::main]
async fn main() {
    tokio::spawn(async move {start_servers().await;});

    tokio::spawn(async move {heartbeat_listener().await;});

    tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
    println!("Shutting down orchestrator...");
}