//! Request-scoped context utilities and actix extractors.

use actix_web::{dev::Payload, web, FromRequest, HttpRequest};
use chrono::{DateTime, Utc};
use futures_util::future::{ready, Ready};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::{self, ApiError};
use crate::models::{SessionRow, User};
use crate::sessions;
use crate::state::AppState;

#[derive(Debug, Clone)]
pub struct AuthUser(pub User, pub Option<SessionRow>);

#[derive(Debug, Clone, Copy)]
pub struct CsrfChecked;

#[derive(Debug, Clone)]
pub struct RequestId(pub String);

impl FromRequest for RequestId {
    type Error = ApiError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let id = req
            .headers()
            .get("X-Request-Id")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::new_v4().simple().to_string());
        ready(Ok(RequestId(id)))
    }
}

impl FromRequest for AuthUser {
    type Error = ApiError;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        Box::pin(async move {
            let state = req
                .app_data::<web::Data<AppState>>()
                .ok_or_else(errors::internal_error)?
                .clone();
            let sid = sessions::read_session_cookie(&req)
                .ok_or_else(|| errors::unauthenticated("Authentication required."))?;
            let session = sessions::get_session(&state.pool, &sid)
                .await
                .map_err(|_| errors::internal_error())?
                .ok_or_else(|| errors::unauthenticated("Authentication required."))?;
            let user: User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
                .bind(&session.user_id)
                .fetch_optional(&state.pool)
                .await
                .map_err(|_| errors::internal_error())?
                .ok_or_else(|| errors::unauthenticated("Authentication required."))?;
            Ok(AuthUser(user, Some(session)))
        })
    }
}

/// Verifies the CSRF token for state-changing requests. The header value
/// must match the `bp_csrf` cookie AND the session row's stored token.
pub async fn require_csrf(req: &HttpRequest, user: &AuthUser) -> Result<(), ApiError> {
    let method = req.method();
    if matches!(*method, actix_web::http::Method::GET | actix_web::http::Method::HEAD | actix_web::http::Method::OPTIONS) {
        return Ok(());
    }
    let header_token = req
        .headers()
        .get("X-CSRF-Token")
        .and_then(|h| h.to_str().ok());
    let cookie_token = sessions::read_csrf_cookie(req);
    let (h, c) = match (header_token, cookie_token) {
        (Some(h), Some(c)) => (h.to_string(), c),
        _ => return Err(errors::csrf_invalid()),
    };
    if !sessions::constant_time_eq(h.as_bytes(), c.as_bytes()) {
        return Err(errors::csrf_invalid());
    }
    let session = user
        .1
        .as_ref()
        .ok_or_else(|| errors::csrf_invalid())?;
    if !sessions::constant_time_eq(session.csrf_token.as_bytes(), h.as_bytes()) {
        return Err(errors::csrf_invalid());
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct AdminUser(pub User);

pub async fn require_admin(user: &AuthUser) -> Result<AdminUser, ApiError> {
    if user.0.role != "admin" {
        return Err(errors::forbidden("Admin role required."));
    }
    Ok(AdminUser(user.0.clone()))
}

pub fn parse_date(s: &str) -> Option<DateTime<Utc>> {
    let s = s.replace("Z", "+00:00");
    DateTime::parse_from_rfc3339(&s)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}

#[derive(Debug, Deserialize)]
pub struct Pagination {
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    100
}

pub fn pool(req: &HttpRequest) -> Result<PgPool, ApiError> {
    req.app_data::<web::Data<AppState>>()
        .map(|s| s.pool.clone())
        .ok_or_else(errors::internal_error)
}
