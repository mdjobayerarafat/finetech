//! Standalone seed binary: `cargo run --bin bankpilot-seed [-- --reset]`.

use bankpilot::{config, db, seed};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .try_init();
    let _ = config::get_settings();
    let pool = db::build_pool().await?;
    let reset = std::env::args().any(|a| a == "--reset");
    if reset {
        seed::reset_and_seed(&pool).await?;
        println!("{{\"ok\": true, \"reset\": true}}");
    } else {
        seed::seed_if_empty(&pool).await?;
        println!("{{\"ok\": true, \"reset\": false}}");
    }
    Ok(())
}
