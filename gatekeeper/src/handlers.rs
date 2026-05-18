use rocket::{get, post};
use rocket::serde::json::Json;
use rocket::http::Status;
use serde::{Serialize, Deserialize};
use rocket::State;
use deadpool_redis::Pool;
use uuid::Uuid;
use crate::redis_pool;
use maxminddb::{Reader, path};

// HEALTH ENDPOINT PART
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
}

#[get("/health")]
pub fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

//LOGIN ENDPOINT PART
#[derive(Deserialize)]
pub struct LoginRequest<'r> {
    pub username: &'r str,
    pub password: &'r str,
}

#[derive(Serialize)]
pub struct ServerInfo {
    pub ip: String,
    pub port: u16,
    pub zone: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub player_id: String,
    pub server: ServerInfo,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

fn map_continent_to_zone(continent_code: &str) -> String {
    match continent_code {
        "EU" => "EU".to_string(),
        "NA" => "NA".to_string(),
        "SA" => "NA".to_string(),
        "AS" | "OC" => "ASIA".to_string(),
        _ => "EU".to_string(), // Zone par défaut si on ne sait pas
    }
}

#[post("/login", data = "<login_data>")]
pub async fn login(
    client_ip: std::net::IpAddr,
    login_data: Json<LoginRequest<'_>>,
    pool: &State<Pool>,
    geo_db: &State<Reader<Vec<u8>>>
) -> Result<Json<LoginResponse>, (Status, Json<ErrorResponse>)> {

    let user = login_data.username;
    let pass = login_data.password;

    // j'ai inverser la condition
    if user.is_empty() || pass != "1234" {
        return Err((Status::Unauthorized, Json(ErrorResponse {
            error: "Nom d'utilisateur ou mot de passe incorrect".to_string()
        })));
    }

    // default
    let mut player_zone = "EU".to_string();

    // check db geo pour deduire la zone
    // Note : 127.0.0.1 = erreur donc par défaut = "EU"
    if let Ok(lookup_result) = geo_db.lookup(client_ip) {
        if let Ok(Some(continent_code)) = lookup_result.decode_path::<String>(&path!["continent", "code"]) {
            // trad
            player_zone = map_continent_to_zone(&continent_code);
        }
    }

    let result = redis_pool::find_available_server(player_zone, pool).await;

    match result {
        Some(server_info) => {
            // Serveur trouvé 
            let player_id = Uuid::new_v4().to_string();

            Ok(Json(LoginResponse {
                player_id,
                server: server_info,
            }))
        }
        None => {
            // renvoie l'erreur 503
            Err((Status::ServiceUnavailable, Json(ErrorResponse {
                error: "No server available".to_string(),
            })))
        }
    }
}