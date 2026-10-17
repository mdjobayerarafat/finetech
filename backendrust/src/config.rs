//! Application configuration loaded from environment variables.
//!
//! Mirrors `backend/app/config.py`.

use std::sync::OnceLock;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    /// Bind host.
    pub host: String,
    /// Bind port.
    pub port: u16,
    /// One of "development" | "production".
    pub app_env: String,
    /// PostgreSQL connection URL, e.g. `postgres://user:pass@host:port/db`.
    pub database_url: String,
    /// HMAC secret for signing session cookies + canonicalisation.
    pub session_secret: String,
    pub session_ttl_seconds: i64,
    pub pending_action_ttl_seconds: i64,
    pub cookie_secure: bool,
    pub openai_api_key: String,
    pub openai_base_url: String,
    pub openai_model: String,
    pub openrouter_referer: String,
    pub openrouter_title: String,
    /// Unsloth base URL (OpenAI-compatible). Empty = disabled.
    pub unsloth_url: String,
    /// Unsloth API key (any non-empty value enables it).
    pub unsloth_api_key: String,
    /// Unsloth model slug.
    pub unsloth_model: String,
    pub daily_limit_fen: i64,
    pub mfa_demo_code: String,
    pub max_failed_mfa: i32,
}

impl Settings {
    pub fn llm_enabled(&self) -> bool {
        !self.openai_api_key.trim().is_empty()
    }

    /// Unsloth is enabled when both the URL and API key are non-empty.
    pub fn unsloth_enabled(&self) -> bool {
        !self.unsloth_url.trim().is_empty() && !self.unsloth_api_key.trim().is_empty()
    }

    pub fn is_production(&self) -> bool {
        self.app_env.eq_ignore_ascii_case("production")
    }
}

static SETTINGS: OnceLock<Settings> = OnceLock::new();

pub fn get_settings() -> &'static Settings {
    SETTINGS.get_or_init(load)
}

pub fn load() -> Settings {
    // Best-effort load of a `.env` file in the repo root or current dir.
    let _ = dotenvy::dotenv();

    let mut s = Settings {
        host: env_or("HOST", "0.0.0.0"),
        port: env_parse("PORT", 8080),
        app_env: env_or("APP_ENV", "development"),
        database_url: env_or("DATABASE_URL", "postgres://postgres:postgres@127.0.0.1:5432/postgres"),
        session_secret: env_or(
            "SESSION_SECRET",
            "dev-only-secret-please-change-in-prod-32bytes!!",
        ),
        session_ttl_seconds: env_parse("SESSION_TTL_SECONDS", 12 * 60 * 60),
        pending_action_ttl_seconds: env_parse("PENDING_ACTION_TTL_SECONDS", 5 * 60),
        cookie_secure: env_parse("COOKIE_SECURE", false),
        openai_api_key: env_or("OPENAI_API_KEY", ""),
        openai_base_url: env_or("OPENAI_BASE_URL", "https://api.openai.com/v1"),
        openai_model: env_or("OPENAI_MODEL", "gpt-4o-mini"),
        openrouter_referer: env_or("OPENROUTER_REFERER", "http://localhost:3000"),
        openrouter_title: env_or("OPENROUTER_TITLE", "BankPilot"),
        // Unsloth (OpenAI-compatible). Accept clean keys and the legacy
        // loose keys from the repo's existing .env file.
        unsloth_url: env_or_first(&[
            "UNSLOTH_URL",
            "UNSLOTH_BASE_URL",
            "unsloth url", // legacy
        ], ""),
        unsloth_api_key: env_or_first(&[
            "UNSLOTH_API_KEY",
            "UNSLOTH_APIKEY",
            "unslothapi", // legacy
        ], ""),
        unsloth_model: env_or_first(&[
            "UNSLOTH_MODEL",
            "model", // legacy — last on purpose: too generic
        ], "unsloth/gemma-4-E2B-it-GGUF:UD-Q4_K_XL"),
        daily_limit_fen: env_parse("DAILY_LIMIT_FEN", 1_000_000),
        mfa_demo_code: env_or("MFA_DEMO_CODE", "000000"),
        max_failed_mfa: env_parse("MAX_FAILED_MFA", 3),
    };

    // Force-secure cookies in production.
    if s.is_production() {
        s.cookie_secure = true;
    }
    s
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

/// Read the first non-empty value among a list of candidate env-var names.
fn env_or_first(keys: &[&str], default: &str) -> String {
    for k in keys {
        if let Ok(v) = std::env::var(k) {
            if !v.trim().is_empty() {
                return v;
            }
        }
    }
    default.to_string()
}

fn env_parse<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}



