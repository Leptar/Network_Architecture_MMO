use deadpool_redis::{Config, Runtime, Pool};

pub fn create_pool() -> Pool {
   let mut cfg = Config::from_url("redis://127.0.0.1:6379");

   cfg.create_pool(Some(Runtime::Tokio1))
        .expect("Erreur fatale : Impossible de créer le pool Redis. Redis est-il lancé via Docker ?")
}