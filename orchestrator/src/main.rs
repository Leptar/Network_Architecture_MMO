use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::net::UdpSocket;
use shared::Heartbeat;

const HOT_SERVERS_MIN: usize = 2;
const CHECK_TIME_SERVERS_AVAILABLE: usize = 2;
const ORCH_PORT: usize = 22555;
const UPDATE_REDIS_TIME: usize = 5;

static DGS_ID: AtomicUsize = AtomicUsize::new(0);

async fn start_servers() {
    for _ in 0..HOT_SERVERS_MIN {
        let id = DGS_ID.fetch_add(1, Ordering::Relaxed);
        let nom_conteneur = format!("dgs-{}", id);
        let orch_addr = format!("ORCH_ADDR=host.docker.internal:{}", ORCH_PORT);

        Command::new("docker")
            .args(&["rm", "-f", &nom_conteneur])
            .output()
            .ok();

        Command::new("docker")
            .args(&[
                "run",
                "--name",
                &nom_conteneur,
                "-e",
                &orch_addr,
                "--add-host=host.docker.internal:host-gateway",
                "dgs-image",
            ])
            .spawn()
            .expect("Failed to start game server");
    }
}

fn stop_servers() {
    let current_id = DGS_ID.load(Ordering::Relaxed);
    for i in 0..current_id {
        let nom_conteneur = format!("dgs-{}", i);
        Command::new("docker")
            .args(&["stop", &nom_conteneur])
            .output()
            .ok();
        Command::new("docker")
            .args(&["rm", &nom_conteneur])
            .output()
            .ok();
        println!("Stopped and removed container {}", nom_conteneur);
    }
}

async fn heartbeat_listener(client: redis::Client) {
    let socket = UdpSocket::bind(("0.0.0.0", ORCH_PORT as u16)).await.expect("Failed to start heartbeat listener");
    println!("Heartbeat listener started on port {}", ORCH_PORT);

    let mut con = client.get_connection().expect("Failed to connect to Redis");

    loop {
        let mut buf = [0; 1024];
        let (len, addr) = socket.recv_from(&mut buf).await.expect("Failed to receive heartbeat");
        let msg = String::from_utf8_lossy(&buf[..len]);
        println!("Received heartbeat from {}: {}", addr, msg);

        if let Ok(heartbeat) = serde_json::from_str::<Heartbeat>(&msg) {
            println!("Parsed heartbeat: {:?}", heartbeat);

            let _: () = redis::cmd("HSET")
                .arg(format!("server:{}", heartbeat.id))
                .arg("port").arg(heartbeat.port)
                .arg("ip").arg(&heartbeat.ip)
                .arg("zone").arg(&heartbeat.zone)
                .arg("player_count").arg(heartbeat.player_count)
                .arg("max_players").arg(heartbeat.max_players)
                .arg("status").arg(if heartbeat.player_count < heartbeat.max_players { "available" } else { "full" })
                .query(&mut con).expect("Failed to update Redis");

            let _: () = redis::cmd("EXPIRE")
                .arg(format!("server:{}", heartbeat.id))
                .arg(15)
                .query(&mut con).expect("Failed to set TTL");
        } else {
            println!("Failed to parse heartbeat, ignoring...");
        }
    }
}

fn count_available_servers(con: &mut redis::Connection) -> usize {
    let keys: Vec<String> = redis::cmd("KEYS")
        .arg("server:*")
        .query(con)
        .expect("Failed to query Redis for server keys");

    let mut count = 0;
    for key in keys {
        let status: String = redis::cmd("HGET")
            .arg(&key)
            .arg("status")
            .query(con)
            .expect("Failed to query Redis for server status");

        if status == "available" {
            count += 1;
        }
    }
    count
}

async fn scaler_loop(client: redis::Client) {
    let mut interval = tokio::time::interval(Duration::from_secs(CHECK_TIME_SERVERS_AVAILABLE as u64));
    let mut con = client.get_connection().expect("Failed to connect to Redis");

    loop {
        interval.tick().await;
        let available_servers = count_available_servers(&mut con);
        println!("Available servers: {}", available_servers);

        for _ in available_servers..HOT_SERVERS_MIN {
            start_servers().await;
        }
    }
}

#[tokio::main]
async fn main() {
    let client = redis::Client::open("redis://127.0.0.1:6379").expect("Failed to create Redis client");
    let client2 = client.clone();

    tokio::spawn(async move { start_servers().await; });
    tokio::spawn(async move { heartbeat_listener(client).await; });
    //tokio::spawn(async move { scaler_loop(client2).await; });

    tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
    stop_servers();
    println!("Shutting down orchestrator...");
}