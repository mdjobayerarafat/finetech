//! Session and CSRF helpers.

use chrono::{DateTime, Duration, Utc};
use hmac::{Hmac, Mac};
use rand::RngCore;
use sha2::Sha256;

use crate::config;
use crate::ids;
use crate::models::{SessionRow, User};

pub const SESSION_COOKIE: &str = "bp_session";
pub const CSRF_COOKIE: &str = "bp_csrf";

pub fn new_csrf_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

pub fn sign(payload: &str) -> String {
    let secret = config::get_settings().session_secret.as_bytes();
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("hmac key");
    mac.update(payload.as_bytes());
    let sig = hex::encode(mac.finalize().into_bytes());
    format!("{}.{}", payload, sig)
}

pub fn unsign(signed: &str) -> Option<String> {
    let (payload, sig) = signed.rsplit_once('.')?;
    let secret = config::get_settings().session_secret.as_bytes();
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("hmac key");
    mac.update(payload.as_bytes());
    let expected = hex::encode(mac.finalize().into_bytes());
    if !constant_time_eq(expected.as_bytes(), sig.as_bytes()) {
        return None;
    }
    Some(payload.to_string())
}

pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

pub async fn create_session(
    pool: &sqlx::PgPool,
    user: &User,
) -> Result<SessionRow, sqlx::Error> {
    let settings = config::get_settings();
    let now = Utc::now();
    let expires = now + Duration::seconds(settings.session_ttl_seconds);
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    // 32 random bytes URL-safe (43 chars after padding stripped).
    use base64::Engine;
    let session_id = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&bytes);
    let csrf = new_csrf_token();
    sqlx::query(
        "INSERT INTO sessions (id, user_id, csrf_token, created_at, expires_at) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(&session_id)
    .bind(&user.id)
    .bind(&csrf)
    .bind(now)
    .bind(expires)
    .execute(pool)
    .await?;
    Ok(SessionRow {
        id: session_id,
        user_id: user.id.clone(),
        csrf_token: csrf,
        created_at: now,
        expires_at: expires,
    })
}

pub async fn get_session(
    pool: &sqlx::PgPool,
    session_id: &str,
) -> Result<Option<SessionRow>, sqlx::Error> {
    let row: Option<SessionRow> = sqlx::query_as("SELECT * FROM sessions WHERE id = $1")
        .bind(session_id)
        .fetch_optional(pool)
        .await?;
    if let Some(r) = row {
        // Normalise naive timestamps to UTC for arithmetic.
        let now = Utc::now();
        let expires: DateTime<Utc> = r.expires_at;
        if expires <= now {
            // Best-effort delete.
            let _ = sqlx::query("DELETE FROM sessions WHERE id = $1")
                .bind(session_id)
                .execute(pool)
                .await;
            return Ok(None);
        }
        Ok(Some(r))
    } else {
        Ok(None)
    }
}

pub async fn destroy_session(pool: &sqlx::PgPool, session_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM sessions WHERE id = $1")
        .bind(session_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub fn attach_cookies(
    response: &mut actix_web::http::header::HeaderMap,
    jar: &mut actix_web::cookie::CookieJar,
    session: &SessionRow,
) {
    let settings = config::get_settings();
    let max_age = settings.session_ttl_seconds as i64;

    let signed = sign(&session.id);
    let session_cookie = actix_web::cookie::Cookie::build(SESSION_COOKIE, signed)
        .path("/")
        .http_only(true)
        .secure(settings.cookie_secure)
        .same_site(actix_web::cookie::SameSite::Lax)
        .max_age(actix_web::cookie::time::Duration::seconds(max_age))
        .finish();
    let csrf_cookie = actix_web::cookie::Cookie::build(CSRF_COOKIE, session.csrf_token.clone())
        .path("/")
        .http_only(false)
        .secure(settings.cookie_secure)
        .same_site(actix_web::cookie::SameSite::Lax)
        .max_age(actix_web::cookie::time::Duration::seconds(max_age))
        .finish();
    jar.add(session_cookie);
    jar.add(csrf_cookie);
    let _ = response;
}

pub fn clear_cookies(jar: &mut actix_web::cookie::CookieJar) {
    jar.remove(actix_web::cookie::Cookie::named(SESSION_COOKIE));
    jar.remove(actix_web::cookie::Cookie::named(CSRF_COOKIE));
}

pub fn read_session_cookie(req: &actix_web::HttpRequest) -> Option<String> {
    let raw = req.cookie(SESSION_COOKIE)?;
    unsign(raw.value())
}

pub fn read_csrf_cookie(req: &actix_web::HttpRequest) -> Option<String> {
    req.cookie(CSRF_COOKIE).map(|c| c.value().to_string())
}

pub fn _silence_unused() {
    // Re-export to keep `ids` in use when the binary is built.
    let _ = ids::new_id;
}
