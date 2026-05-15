use rocket::{launch, routes};

mod handlers;
mod redis_pool;

#[launch]
fn rocket() -> _ {
    let redis_pool = redis_pool::create_pool();

    rocket::build()
        .manage(redis_pool)
        .mount("/", routes![
            handlers::health,
            handlers::login
        ])
}
