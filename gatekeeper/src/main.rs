use rocket::{launch, routes};
use maxminddb::Reader;
mod handlers;
mod redis_pool;

#[launch]
fn rocket() -> _ {
    let redis_pool = redis_pool::create_pool();

    // db pour deduire la region a partir de l'ip
    let geo_db = Reader::open_readfile("GeoLite2-Country.mmdb")
        .expect("Erreur fatale : Impossible de charger la base GeoIP. Le fichier .mmdb est-il présent ?");

    rocket::build()
        .manage(redis_pool)
        .manage(geo_db)
        .mount("/", routes![
            handlers::health,
            handlers::login
        ])
}
