use rocket::{get, post};
use rocket::serde::json::Json;
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
pub fn login(login_data: Json<LoginRequest<'_>>) -> Json<String> {
    Json(format!("Salut {}, j'ai bien reçu ton mot de passe.", login_data.username))
}