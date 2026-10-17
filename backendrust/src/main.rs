//! BankPilot backend entry point.

use actix_cors::Cors;
use actix_web::{
    middleware::{DefaultHeaders, Logger},
    web, App, HttpServer,
};
use log::info;

use bankpilot::{config, db, routes, seed, state};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let settings = config::get_settings();
    let pool = match db::build_pool().await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("failed to build DB pool: {e}");
            std::process::exit(1);
        }
    };
    if let Err(e) = seed::seed_if_empty(&pool).await {
        eprintln!("seed warning: {e}");
    }
    let state = state::AppState { pool: pool.clone() };
    let host = settings.host.clone();
    let port = settings.port;
    info!("BankPilot Rust backend listening on {host}:{port}");
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .wrap(Logger::default())
            .wrap(DefaultHeaders::new().add(("X-Service", "bankpilot-rust")))
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header()
                    .supports_credentials()
                    .max_age(3600),
            )
            .configure(routes::configure)
    })
    .bind((host, port))?
    .run()
    .await
}
