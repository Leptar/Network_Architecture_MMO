use rocket::{get, post};
use rocket::serde::json::Json;
use rocket::http::Status;
use serde::{Serialize, Deserialize};
use rocket::State;
use deadpool_redis::Pool;
use uuid::Uuid;
use crate::redis_pool;

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

fn get_zone_from_ip(ip: std::net::IpAddr) -> String {
    // stub : juste 
     "EU".to_string()
}

#[post("/login", data = "<login_data>")]
pub async fn login(
    client_ip: std::net::IpAddr,
    login_data: Json<LoginRequest<'_>>,
    pool: &State<Pool>
) -> Result<Json<LoginResponse>, (Status, Json<ErrorResponse>)> {

    let user = login_data.username;
    let pass = login_data.password;

    // j'ai inverser la condition
    if user.is_empty() || pass != "1234" {
        return Err((Status::Unauthorized, Json(ErrorResponse {
            error: "Nom d'utilisateur ou mot de passe incorrect".to_string()
        })));
    }

    let result =
        redis_pool::find_available_server(get_zone_from_ip(client_ip),pool).await;

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