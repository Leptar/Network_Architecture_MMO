use rocket::{launch, routes};

mod handlers;
mod redis_pool;

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![
            handlers::health,
            handlers::login
        ])
}
