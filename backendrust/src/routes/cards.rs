//! Card management routes (Scenario 6, stretch).

use actix_web::{web, HttpResponse};
use chrono::Utc;
use rand::Rng;
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;

use crate::audit::{self, AppendEventParams};
use crate::ctx::{require_csrf, AuthUser, RequestId};
use crate::errors::{self, ApiError};
use crate::ids;
use crate::models::Card;
use crate::money::coerce_amount_fen;
use crate::policy;

use crate::state::AppState;
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/cards", web::get().to(list_cards))
        .route("/cards", web::post().to(issue_card))
        .route("/cards/{id}/freeze", web::post().to(freeze))
        .route("/cards/{id}/unfreeze", web::post().to(unfreeze))
        .route("/cards/{id}/cancel", web::post().to(cancel))
        .route("/cards/{id}/report-lost", web::post().to(report_lost))
        .route("/cards/{id}/limits", web::patch().to(update_limits));
}

async fn list_cards(
    state: web::Data<AppState>,
    auth: AuthUser,
) -> Result<HttpResponse, ApiError> {
    let user = auth.0;
    let rows: Vec<Card> = sqlx::query_as(
        "SELECT * FROM cards WHERE user_id = $1 ORDER BY created_at ASC",
    )
    .bind(&user.id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    let out: Vec<_> = rows.iter().map(card_out).collect();
    Ok(HttpResponse::Ok().json(out))
}

#[derive(Deserialize)]
pub struct CardIssueIn {
    #[serde(default = "default_name")]
    pub display_name: String,
    pub single_tx_limit_cny: Option<serde_json::Value>,
    pub monthly_limit_cny: Option<serde_json::Value>,
}
fn default_name() -> String {
    "Virtual card".to_string()
}

async fn issue_card(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    body: web::Json<CardIssueIn>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let user = auth.0;
    let body = body.into_inner();
    let single = body
        .single_tx_limit_cny
        .as_ref()
        .map(coerce_amount_fen)
        .transpose()
        .map_err(|e| errors::schema_invalid(&e))?
        .unwrap_or(50_000);
    let monthly = body
        .monthly_limit_cny
        .as_ref()
        .map(coerce_amount_fen)
        .transpose()
        .map_err(|e| errors::schema_invalid(&e))?
        .unwrap_or(1_000_000);
    let masked = format!("****{:04}", rand::thread_rng().gen_range(0..10000));
    let id = ids::new_id("card");
    let now = Utc::now();
    sqlx::query(
        "INSERT INTO cards (id, user_id, display_name, masked_number, expiry_month, expiry_year, status, single_tx_limit_fen, monthly_limit_fen, created_at)
         VALUES ($1, $2, $3, $4, 12, 2030, 'active', $5, $6, $7)",
    )
    .bind(&id)
    .bind(&user.id)
    .bind(&body.display_name)
    .bind(&masked)
    .bind(single)
    .bind(monthly)
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    audit::append_event(
        &state.pool,
        AppendEventParams {
            action: "card.issue",
            actor_user_id: Some(&user.id),
            session: None,
            request_id: Some(&rid.0),
            workflow_id: None,
            node_id: None,
            tool: Some("cards.issue"),
            parameters_masked: Some(json!({ "card_id": id, "display_name": body.display_name })),
            risk_tier: Some(policy::decide_card_action("issue").tier.as_str()),
            policy_reason: None,
            confirmation_status: None,
            mfa_status: None,
            result: "ok",
            error_code: None,
        },
    )
    .await
    .ok();
    let card: Card = sqlx::query_as("SELECT * FROM cards WHERE id = $1")
        .bind(&id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    Ok(HttpResponse::Ok().json(card_out(&card)))
}

async fn freeze(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let user = auth.0;
    let id = path.into_inner();
    let card = fetch_card(&state.pool, &user.id, &id).await?;
    if card.status == "cancelled" {
        return Err(errors::ApiError::new("schema_invalid", "Card is cancelled.", 409));
    }
    sqlx::query("UPDATE cards SET status = 'frozen' WHERE id = $1")
        .bind(&id)
        .execute(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    card_audit(&state.pool, &user.id, &rid.0, "card.freeze", "cards.freeze", "freeze", &id)
        .await
        .ok();
    let card: Card = sqlx::query_as("SELECT * FROM cards WHERE id = $1")
        .bind(&id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    Ok(HttpResponse::Ok().json(card_out(&card)))
}

async fn unfreeze(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let user = auth.0;
    let id = path.into_inner();
    let _ = fetch_card(&state.pool, &user.id, &id).await?;
    sqlx::query("UPDATE cards SET status = 'active' WHERE id = $1")
        .bind(&id)
        .execute(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    card_audit(&state.pool, &user.id, &rid.0, "card.unfreeze", "cards.unfreeze", "unfreeze", &id)
        .await
        .ok();
    let card: Card = sqlx::query_as("SELECT * FROM cards WHERE id = $1")
        .bind(&id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    Ok(HttpResponse::Ok().json(card_out(&card)))
}

async fn cancel(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let user = auth.0;
    let id = path.into_inner();
    let _ = fetch_card(&state.pool, &user.id, &id).await?;
    sqlx::query("UPDATE cards SET status = 'cancelled' WHERE id = $1")
        .bind(&id)
        .execute(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    card_audit(&state.pool, &user.id, &rid.0, "card.cancel", "cards.cancel", "cancel", &id)
        .await
        .ok();
    let card: Card = sqlx::query_as("SELECT * FROM cards WHERE id = $1")
        .bind(&id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    Ok(HttpResponse::Ok().json(card_out(&card)))
}

async fn report_lost(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let user = auth.0;
    let id = path.into_inner();
    let _ = fetch_card(&state.pool, &user.id, &id).await?;
    sqlx::query("UPDATE cards SET status = 'lost' WHERE id = $1")
        .bind(&id)
        .execute(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    card_audit(
        &state.pool,
        &user.id,
        &rid.0,
        "card.report_lost",
        "cards.report_lost",
        "report_lost",
        &id,
    )
    .await
    .ok();
    let card: Card = sqlx::query_as("SELECT * FROM cards WHERE id = $1")
        .bind(&id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    Ok(HttpResponse::Ok().json(card_out(&card)))
}

#[derive(Deserialize)]
pub struct CardLimitsIn {
    pub single_tx_limit_cny: Option<serde_json::Value>,
    pub monthly_limit_cny: Option<serde_json::Value>,
}

async fn update_limits(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    body: web::Json<CardLimitsIn>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let user = auth.0;
    let id = path.into_inner();
    let _ = fetch_card(&state.pool, &user.id, &id).await?;
    let body = body.into_inner();
    if let Some(s) = body.single_tx_limit_cny {
        let v = coerce_amount_fen(&s).map_err(|e| errors::schema_invalid(&e))?;
        sqlx::query("UPDATE cards SET single_tx_limit_fen = $1 WHERE id = $2")
            .bind(v)
            .bind(&id)
            .execute(&state.pool)
            .await
            .map_err(|_| errors::internal_error())?;
    }
    if let Some(s) = body.monthly_limit_cny {
        let v = coerce_amount_fen(&s).map_err(|e| errors::schema_invalid(&e))?;
        sqlx::query("UPDATE cards SET monthly_limit_fen = $1 WHERE id = $2")
            .bind(v)
            .bind(&id)
            .execute(&state.pool)
            .await
            .map_err(|_| errors::internal_error())?;
    }
    card_audit(
        &state.pool,
        &user.id,
        &rid.0,
        "card.update_limits",
        "cards.update_limits",
        "update_limits",
        &id,
    )
    .await
    .ok();
    let card: Card = sqlx::query_as("SELECT * FROM cards WHERE id = $1")
        .bind(&id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    Ok(HttpResponse::Ok().json(card_out(&card)))
}

fn card_out(c: &Card) -> serde_json::Value {
    json!({
        "id": c.id,
        "display_name": c.display_name,
        "masked_number": c.masked_number,
        "expiry_month": c.expiry_month,
        "expiry_year": c.expiry_year,
        "status": c.status,
        "single_tx_limit_fen": c.single_tx_limit_fen,
        "monthly_limit_fen": c.monthly_limit_fen,
    })
}

async fn fetch_card(pool: &PgPool, user_id: &str, id: &str) -> Result<Card, ApiError> {
    let row: Option<Card> = sqlx::query_as("SELECT * FROM cards WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|_| errors::internal_error())?;
    let row = row.ok_or_else(|| errors::not_found("Card not found."))?;
    if row.user_id != user_id {
        return Err(errors::not_found("Card not found."));
    }
    Ok(row)
}

async fn card_audit(
    pool: &PgPool,
    user_id: &str,
    rid: &str,
    action: &str,
    tool: &str,
    kind: &str,
    card_id: &str,
) -> Result<(), sqlx::Error> {
    audit::append_event(
        pool,
        AppendEventParams {
            action,
            actor_user_id: Some(user_id),
            session: None,
            request_id: Some(rid),
            workflow_id: None,
            node_id: None,
            tool: Some(tool),
            parameters_masked: Some(json!({ "card_id": card_id, "kind": kind })),
            risk_tier: Some(policy::decide_card_action(kind).tier.as_str()),
            policy_reason: None,
            confirmation_status: None,
            mfa_status: None,
            result: "ok",
            error_code: None,
        },
    )
    .await?;
    Ok(())
}
