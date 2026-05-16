use deadpool_redis::{Config, Runtime, Pool};
use deadpool_redis::redis::AsyncCommands;
use crate::handlers::ServerInfo;

pub async fn find_available_server(pool: &Pool) -> Option<ServerInfo> {
   // connexion a la pool
   let mut conn = pool.get().await.ok()?;

   // je récupère les clés
   let keys: Vec<String> = conn.keys("server:*").await.unwrap_or_default();

   for key in keys {
      // je regarde le status
      let status: Option<String> = conn.hget(&key, "status").await.ok();

      if let Some(s) = status {
         if s == "available" {
            let ip: String = conn.hget(&key, "ip").await.unwrap_or_default();
            let port: u16 = conn.hget(&key, "port").await.unwrap_or_default();
            let zone: String = conn.hget(&key, "zone").await.unwrap_or_default();

            return Some(ServerInfo { ip, port, zone });
         }
      }
   }

   // Rien trouver
   None
}

pub fn create_pool() -> Pool {
   let mut cfg = Config::from_url("redis://127.0.0.1:6379");

   cfg.create_pool(Some(Runtime::Tokio1))
        .expect("Erreur fatale : Impossible de créer le pool Redis. Redis est-il lancé via Docker ?")
}