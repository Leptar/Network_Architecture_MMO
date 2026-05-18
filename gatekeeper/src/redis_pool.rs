use deadpool_redis::{Config, Runtime, Pool};
use deadpool_redis::redis::AsyncCommands;
use rand::RngExt;
use crate::handlers::ServerInfo;

pub async fn find_available_server(zone: String, pool: &Pool) -> Option<ServerInfo> {
   // connexion a la pool
   let mut conn = pool.get().await.ok()?;

   // je récupère les clés
   let keys: Vec<String> = conn.keys("server:*").await.unwrap_or_default();
   let mut valid_servers: Vec<ServerInfo> = Vec::new();

   for key in keys {
      // je regarde le status et la zone
      let status: Option<String> = conn.hget(&key, "status").await.ok();
      let server_zone: Option<String> = conn.hget(&key, "zone").await.ok();

      if let (Some(s), Some(z)) = (status, server_zone) {
         if s == "available" && z == zone{
            let ip: String = conn.hget(&key, "ip").await.unwrap_or_default();
            let port: u16 = conn.hget(&key, "port").await.unwrap_or_default();
            let zone: String = conn.hget(&key, "zone").await.unwrap_or_default();

            valid_servers.push(ServerInfo{ip, port, zone});
         }
      }
   }

   if !valid_servers.is_empty() {
      let mut rng = rand::rng();
      let random_index = rng.random_range(0..valid_servers.len());
      return Some(valid_servers.remove(random_index));
   }

   // Rien trouver
   None
}

pub fn create_pool() -> Pool {
   let mut cfg = Config::from_url("redis://127.0.0.1:6379");

   cfg.create_pool(Some(Runtime::Tokio1))
        .expect("Erreur fatale : Impossible de créer le pool Redis. Redis est-il lancé via Docker ?")
}