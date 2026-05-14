use rocket::{get, post};
use rocket::serde::json::Json;
use rocket::http::Status;
use serde::{Serialize, Deserialize};

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

#[post("/login", data = "<login_data>")]
pub fn login(login_data: Json<LoginRequest<'_>>) -> Result<Json<LoginResponse>, (Status, Json<ErrorResponse>)> {
    let user = login_data.username;
    let pass = login_data.password;

    // Stub : je verifie pas reelement quoi que se soit just le user doit pas etre vide et le pass
    // c'est 1234.
    if !user.is_empty() && pass == "1234" {
        Ok(Json(LoginResponse {
            player_id: "un-id-unique-temporaire".to_string(),
            server: ServerInfo {
                ip: "127.0.0.1".to_string(),
                port: 7001,
                zone: "zone_A".to_string(),
            },
        }))
    } else {
        Err((Status::Unauthorized, Json(ErrorResponse {
            error: "Nom d'utilisateur ou mot de passe incorrect".to_string()
        })))
    }
}