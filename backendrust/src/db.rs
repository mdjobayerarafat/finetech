//! PostgreSQL pool + helpers.

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{ConnectOptions, PgPool};
use std::str::FromStr;
use std::time::Duration;

use crate::config;

pub async fn build_pool() -> Result<PgPool, sqlx::Error> {
    let url = &config::get_settings().database_url;
    let mut opts = PgConnectOptions::from_str(url)
        .map_err(|e| sqlx::Error::Configuration(Box::new(e)))?;
    // Suppress INFO-level sqlx logs; we have our own request log.
    opts = opts.log_statements(log::LevelFilter::Debug);

    let pool = PgPoolOptions::new()
        .max_connections(8)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(10))
        .connect_with(opts)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}
