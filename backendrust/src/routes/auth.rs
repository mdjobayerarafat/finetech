//! Auth + /me routes.

use actix_web::{
    cookie::{Cookie, CookieJar, SameSite},
    web, HttpRequest, HttpResponse,
};
use chrono::{Duration, Utc};
use serde::Deserialize;
use serde_json::json;

use crate::audit::{self, AppendEventParams};
use crate::config;
use crate::ctx::{require_csrf, AuthUser, RequestId};
use crate::errors::{self, ApiError};
use crate::ids;
use crate::models::{SessionRow, User};
use crate::security;
use crate::sessions;
use crate::state::AppState;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/auth/register", web::post().to(register))
        .route("/auth/login", web::post().to(login))
        .route("/auth/logout", web::post().to(logout))
        .route("/auth/csrf", web::get().to(get_csrf))
        .route("/me", web::get().to(get_me))
        .route("/me/language", web::post().to(update_language));
}

#[derive(Deserialize)]
pub struct RegisterIn {
    pub username: String,
    pub password: String,
    pub display_name: String,
    #[serde(default = "default_lang")]
    pub preferred_language: String,
}
fn default_lang() -> String {
    "en".to_string()
}

async fn register(
    state: web::Data<AppState>,
    body: web::Json<RegisterIn>,
    rid: RequestId,
) -> Result<HttpResponse, ApiError> {
    let body = body.into_inner();
    if body.username.is_empty() || body.password.is_empty() {
        return Err(errors::schema_invalid("username and password are required"));
    }
    let existing: Option<User> =
        sqlx::query_as("SELECT * FROM users WHERE username = $1")
            .bind(&body.username)
            .fetch_optional(&state.pool)
            .await
            .map_err(|_| errors::internal_error())?;
    if existing.is_some() {
        return Err(errors::ApiError::new(
            "schema_invalid",
            "username already taken",
            400,
        ));
    }
    let hash = security::hash_password(&body.password).map_err(|_| errors::internal_error())?;
    let id = ids::new_id("user");
    let now = Utc::now();
    sqlx::query(
        "INSERT INTO users (id, username, password_hash, display_name, preferred_language, role, safety_lock_state, failed_mfa_count, risk_tolerance, horizon, created_at)
         VALUES ($1, $2, $3, $4, $5, 'customer', 'unlocked', 0, 'medium', 'medium', $6)",
    )
    .bind(&id)
    .bind(&body.username)
    .bind(&hash)
    .bind(&body.display_name)
    .bind(&body.preferred_language)
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    let user = User {
        id: id.clone(),
        username: body.username.clone(),
        password_hash: hash,
        display_name: body.display_name.clone(),
        preferred_language: body.preferred_language.clone(),
        role: "customer".to_string(),
        safety_lock_state: "unlocked".to_string(),
        safety_lock_reason: None,
        failed_mfa_count: 0,
        risk_tolerance: "medium".to_string(),
        horizon: "medium".to_string(),
        created_at: now,
    };
    audit::append_event(
        &state.pool,
        AppendEventParams {
            action: "auth.register",
            actor_user_id: Some(&user.id),
            session: None,
            request_id: Some(&rid.0),
            workflow_id: None,
            node_id: None,
            tool: None,
            parameters_masked: Some(json!({ "username": user.username })),
            risk_tier: None,
            policy_reason: None,
            confirmation_status: None,
            mfa_status: None,
            result: "ok",
            error_code: None,
        },
    )
    .await
    .ok();
    Ok(HttpResponse::Ok().json(json!({
        "id": user.id,
        "username": user.username,
        "display_name": user.display_name,
    })))
}

#[derive(Deserialize)]
pub struct LoginIn {
    pub username: String,
    pub password: String,
}

async fn login(
    state: web::Data<AppState>,
    body: web::Json<LoginIn>,
    rid: RequestId,
) -> Result<HttpResponse, ApiError> {
    let body = body.into_inner();
    let user: Option<User> = sqlx::query_as("SELECT * FROM users WHERE username = $1")
        .bind(&body.username)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| {
            log::error!("login: SELECT users failed: {e}");
            errors::internal_error()
        })?;
    let user = match user {
        Some(u) => u,
        None => {
            audit::append_event(
                &state.pool,
                AppendEventParams {
                    action: "auth.login",
                    actor_user_id: None,
                    session: None,
                    request_id: Some(&rid.0),
                    workflow_id: None,
                    node_id: None,
                    tool: None,
                    parameters_masked: Some(json!({ "username": body.username })),
                    risk_tier: None,
                    policy_reason: None,
                    confirmation_status: None,
                    mfa_status: None,
                    result: "error",
                    error_code: Some("unauthenticated"),
                },
            )
            .await
            .ok();
            return Err(errors::unauthenticated("Invalid username or password."));
        }
    };
    if !security::verify_password(&body.password, &user.password_hash) {
        audit::append_event(
            &state.pool,
            AppendEventParams {
                action: "auth.login",
                actor_user_id: Some(&user.id),
                session: None,
                request_id: Some(&rid.0),
                workflow_id: None,
                node_id: None,
                tool: None,
                parameters_masked: Some(json!({ "username": body.username })),
                risk_tier: None,
                policy_reason: None,
                confirmation_status: None,
                mfa_status: None,
                result: "error",
                error_code: Some("unauthenticated"),
            },
        )
        .await
        .ok();
        return Err(errors::unauthenticated("Invalid username or password."));
    }
    if user.safety_lock_state == "locked" {
        audit::append_event(
            &state.pool,
            AppendEventParams {
                action: "auth.login",
                actor_user_id: Some(&user.id),
                session: None,
                request_id: Some(&rid.0),
                workflow_id: None,
                node_id: None,
                tool: None,
                parameters_masked: Some(json!({ "username": body.username })),
                risk_tier: None,
                policy_reason: None,
                confirmation_status: None,
                mfa_status: None,
                result: "blocked",
                error_code: Some("safety_locked"),
            },
        )
        .await
        .ok();
        return Err(errors::safety_locked(user.safety_lock_reason.as_deref()));
    }
    let session = sessions::create_session(&state.pool, &user)
        .await
        .map_err(|e| {
            log::error!("login: create_session failed: {e}");
            errors::internal_error()
        })?;
    let mut jar = CookieJar::new();
    let settings = config::get_settings();
    let signed = sessions::sign(&session.id);
    jar.add(
        Cookie::build(sessions::SESSION_COOKIE, signed)
            .path("/")
            .http_only(true)
            .secure(settings.cookie_secure)
            .same_site(SameSite::Lax)
            .max_age(actix_web::cookie::time::Duration::seconds(
                settings.session_ttl_seconds as i64,
            ))
            .finish(),
    );
    jar.add(
        Cookie::build(sessions::CSRF_COOKIE, session.csrf_token.clone())
            .path("/")
            .http_only(false)
            .secure(settings.cookie_secure)
            .same_site(SameSite::Lax)
            .max_age(actix_web::cookie::time::Duration::seconds(
                settings.session_ttl_seconds as i64,
            ))
            .finish(),
    );
    audit::append_event(
        &state.pool,
        AppendEventParams {
            action: "auth.login",
            actor_user_id: Some(&user.id),
            session: Some(&session),
            request_id: Some(&rid.0),
            workflow_id: None,
            node_id: None,
            tool: None,
            parameters_masked: Some(json!({ "username": body.username })),
            risk_tier: None,
            policy_reason: None,
            confirmation_status: None,
            mfa_status: None,
            result: "ok",
            error_code: None,
        },
    )
    .await
    .ok();
    let mut resp = HttpResponse::Ok().json(json!({
        "user_id": user.id,
        "role": user.role,
        "csrf_token": session.csrf_token,
    }));
    for c in jar.delta() {
        resp.add_cookie(c).ok();
    }
    Ok(resp)
}

async fn logout(
    state: web::Data<AppState>,
    req: HttpRequest,
    auth: AuthUser,
) -> Result<HttpResponse, ApiError> {
    if let Some(sid) = sessions::read_session_cookie(&req) {
        sessions::destroy_session(&state.pool, &sid)
            .await
            .ok();
    }
    let mut jar = CookieJar::new();
    jar.remove(Cookie::named(sessions::SESSION_COOKIE));
    jar.remove(Cookie::named(sessions::CSRF_COOKIE));
    let mut resp = HttpResponse::Ok().json(json!({ "ok": true }));
    for c in jar.delta() {
        resp.add_cookie(c).ok();
    }
    let _ = auth;
    Ok(resp)
}

async fn get_csrf(state: web::Data<AppState>, req: HttpRequest) -> Result<HttpResponse, ApiError> {
    let sid = sessions::read_session_cookie(&req)
        .ok_or_else(|| errors::unauthenticated("Authentication required."))?;
    let session: Option<SessionRow> = sqlx::query_as("SELECT * FROM sessions WHERE id = $1")
        .bind(&sid)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    let session = session.ok_or_else(|| errors::unauthenticated("Authentication required."))?;
    let now = Utc::now();
    if session.expires_at <= now {
        return Err(errors::unauthenticated("Authentication required."));
    }
    Ok(HttpResponse::Ok().json(json!({ "csrf_token": session.csrf_token })))
}

async fn get_me(auth: AuthUser) -> Result<HttpResponse, ApiError> {
    let user = auth.0;
    Ok(HttpResponse::Ok().json(json!({
        "id": user.id,
        "username": user.username,
        "display_name": user.display_name,
        "preferred_language": user.preferred_language,
        "role": user.role,
        "safety_lock_state": user.safety_lock_state,
        "safety_lock_reason": user.safety_lock_reason,
        "failed_mfa_count": user.failed_mfa_count,
        "risk_tolerance": user.risk_tolerance,
        "horizon": user.horizon,
    })))
}

#[derive(Deserialize)]
pub struct LanguageIn {
    /// "en" or "zh". Anything else falls back to "en".
    pub language: String,
}

/// POST /api/v1/me/language
/// Persists the user's preferred interface language. Used by the
/// LanguageButton in the app shell so the choice survives reloads and
/// (more importantly) so the agent pipeline knows which language to
/// answer in when the LLM isn't reachable and the stub takes over.
async fn update_language(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: HttpRequest,
    body: web::Json<LanguageIn>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let user = auth.0;
    let requested = body.into_inner().language;
    let lang = match requested.to_lowercase().as_str() {
        "zh" | "zh-cn" | "zh-hans" | "chinese" | "中文" => "zh".to_string(),
        _ => "en".to_string(),
    };
    sqlx::query("UPDATE users SET preferred_language = $1 WHERE id = $2")
        .bind(&lang)
        .bind(&user.id)
        .execute(&state.pool)
        .await
        .map_err(|e| {
            log::error!("update_language: UPDATE users failed: {e}");
            errors::internal_error()
        })?;
    audit::append_event(
        &state.pool,
        AppendEventParams {
            action: "user.update_language",
            actor_user_id: Some(&user.id),
            session: None,
            request_id: Some(&rid.0),
            workflow_id: None,
            node_id: None,
            tool: None,
            parameters_masked: Some(json!({ "language": lang })),
            risk_tier: None,
            policy_reason: None,
            confirmation_status: None,
            mfa_status: None,
            result: "ok",
            error_code: None,
        },
    )
    .await
    .ok();
    Ok(HttpResponse::Ok().json(json!({
        "id": user.id,
        "preferred_language": lang,
    })))
}
