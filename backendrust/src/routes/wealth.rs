//! Wealth catalog + holdings routes (Scenario 5, stretch).

use actix_web::{web, HttpResponse};
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;

use crate::audit::{self, AppendEventParams};
use crate::ctx::{require_csrf, AuthUser, RequestId};
use crate::errors::{self, ApiError};
use crate::ids;
use crate::models::{Account, WealthHolding, WealthProduct};
use crate::money::coerce_amount_fen;
use crate::planner;
use crate::policy::{self, DecideResult};
use crate::safety;

use crate::state::AppState;
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/wealth/products", web::get().to(list_products))
        .route("/wealth/products/{id}", web::get().to(get_product))
        .route("/wealth/holdings", web::get().to(list_holdings))
        .route("/wealth/subscribe", web::post().to(subscribe))
        .route("/wealth/redeem", web::post().to(redeem));
}

async fn list_products(
    state: web::Data<AppState>,
    _auth: AuthUser,
) -> Result<HttpResponse, ApiError> {
    let rows: Vec<WealthProduct> = sqlx::query_as(
        "SELECT * FROM wealth_products ORDER BY name ASC",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    let out: Vec<_> = rows.iter().map(product_out).collect();
    Ok(HttpResponse::Ok().json(out))
}

async fn get_product(
    state: web::Data<AppState>,
    _auth: AuthUser,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let id = path.into_inner();
    let row: Option<WealthProduct> = sqlx::query_as("SELECT * FROM wealth_products WHERE id = $1")
        .bind(&id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    let row = row.ok_or_else(|| errors::not_found("Product not found."))?;
    Ok(HttpResponse::Ok().json(product_out(&row)))
}

fn product_out(p: &WealthProduct) -> serde_json::Value {
    json!({
        "id": p.id,
        "name": p.name,
        "category": p.category,
        "risk_level": p.risk_level,
        "expected_annual_return_pct": p.expected_annual_return_pct,
        "min_subscription_fen": p.min_subscription_fen,
        "suitability_required": p.suitability_required,
    })
}

async fn list_holdings(
    state: web::Data<AppState>,
    auth: AuthUser,
) -> Result<HttpResponse, ApiError> {
    let user = auth.0;
    let rows: Vec<WealthHolding> = sqlx::query_as(
        "SELECT * FROM wealth_holdings WHERE user_id = $1 ORDER BY subscribed_at DESC",
    )
    .bind(&user.id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    let out: Vec<_> = rows
        .into_iter()
        .map(|h| {
            json!({
                "id": h.id,
                "product_id": h.product_id,
                "amount_fen": h.amount_fen,
                "subscribed_at": h.subscribed_at,
                "status": h.status,
            })
        })
        .collect();
    Ok(HttpResponse::Ok().json(out))
}

#[derive(Deserialize)]
pub struct SubscribeIn {
    pub product_id: String,
    pub amount: serde_json::Value,
    pub account_id: Option<String>,
}

async fn subscribe(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    body: web::Json<SubscribeIn>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let user = auth.0;
    safety::assert_unlocked(&user)?;
    let body = body.into_inner();
    let product: Option<WealthProduct> = sqlx::query_as("SELECT * FROM wealth_products WHERE id = $1")
        .bind(&body.product_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    let product = product.ok_or_else(|| errors::not_found("Product not found."))?;
    let decision = match policy::decide_wealth_subscribe(&product, &user) {
        DecideResult::Decision(d) => d,
        DecideResult::SuitabilityDenied(r) => return Err(errors::suitability_failed(&r)),
    };
    let amount_fen = coerce_amount_fen(&body.amount).map_err(|e| errors::schema_invalid(&e))?;
    if amount_fen < product.min_subscription_fen {
        return Err(errors::ApiError::new(
            "below_minimum",
            "Amount below product minimum.",
            422,
        ));
    }
    let account = match planner::resolve_account(&state.pool, &user, body.account_id.as_deref()).await {
        Ok(a) => a,
        Err(_) => return Err(errors::not_found("Account not found.")),
    };
    if account.available_balance_fen < amount_fen {
        return Err(errors::ApiError::new(
            "insufficient_funds",
            "Not enough balance.",
            422,
        ));
    }
    let mut tx = state.pool
        .begin()
        .await
        .map_err(|_| errors::internal_error())?;
    let holding_id = ids::new_id("hold");
    let now = Utc::now();
    sqlx::query(
        "INSERT INTO wealth_holdings (id, user_id, product_id, amount_fen, subscribed_at, status, created_at) VALUES ($1, $2, $3, $4, $5, 'active', $6)",
    )
    .bind(&holding_id)
    .bind(&user.id)
    .bind(&product.id)
    .bind(amount_fen)
    .bind(now)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|_| errors::internal_error())?;
    let tx_id = ids::new_id("tx");
    sqlx::query(
        "INSERT INTO transactions (id, account_id, amount_fen, direction, category, merchant, memo, occurred_at, status) VALUES ($1, $2, $3, 'debit', 'wealth', $4, '', $5, 'posted')",
    )
    .bind(&tx_id)
    .bind(&account.id)
    .bind(-amount_fen)
    .bind(&product.name)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|_| errors::internal_error())?;
    sqlx::query("UPDATE accounts SET available_balance_fen = available_balance_fen - $1 WHERE id = $2")
        .bind(amount_fen)
        .bind(&account.id)
        .execute(&mut *tx)
        .await
        .map_err(|_| errors::internal_error())?;
    tx.commit()
        .await
        .map_err(|_| errors::internal_error())?;
    audit::append_event(
        &state.pool,
        AppendEventParams {
            action: "wealth.subscribe",
            actor_user_id: Some(&user.id),
            session: None,
            request_id: Some(&rid.0),
            workflow_id: None,
            node_id: None,
            tool: Some("wealth.subscribe"),
            parameters_masked: Some(json!({ "product_id": product.id, "amount_fen": amount_fen })),
            risk_tier: Some(&decision.tier),
            policy_reason: Some(&decision.reason),
            confirmation_status: None,
            mfa_status: None,
            result: "ok",
            error_code: None,
        },
    )
    .await
    .ok();
    Ok(HttpResponse::Ok().json(json!({
        "holding_id": holding_id,
        "product_id": product.id,
        "amount_fen": amount_fen,
        "risk_tier": decision.tier,
        "decision_reason": decision.reason,
    })))
}

#[derive(Deserialize)]
pub struct RedeemIn {
    pub holding_id: String,
    pub account_id: Option<String>,
}

async fn redeem(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    body: web::Json<RedeemIn>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let user = auth.0;
    safety::assert_unlocked(&user)?;
    let body = body.into_inner();
    let holding: Option<WealthHolding> = sqlx::query_as("SELECT * FROM wealth_holdings WHERE id = $1")
        .bind(&body.holding_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    let holding = holding.ok_or_else(|| errors::not_found("Holding not found."))?;
    if holding.user_id != user.id {
        return Err(errors::not_found("Holding not found."));
    }
    if holding.status != "active" {
        return Ok(HttpResponse::Ok().json(json!({
            "holding_id": holding.id,
            "status": holding.status,
        })));
    }
    let account = match planner::resolve_account(&state.pool, &user, body.account_id.as_deref()).await {
        Ok(a) => a,
        Err(_) => return Err(errors::not_found("Account not found.")),
    };
    let mut tx = state.pool
        .begin()
        .await
        .map_err(|_| errors::internal_error())?;
    sqlx::query("UPDATE wealth_holdings SET status = 'redeemed' WHERE id = $1")
        .bind(&holding.id)
        .execute(&mut *tx)
        .await
        .map_err(|_| errors::internal_error())?;
    let tx_id = ids::new_id("tx");
    let now = Utc::now();
    sqlx::query(
        "INSERT INTO transactions (id, account_id, amount_fen, direction, category, merchant, memo, occurred_at, status) VALUES ($1, $2, $3, 'credit', 'wealth', 'wealth-redeem', '', $4, 'posted')",
    )
    .bind(&tx_id)
    .bind(&account.id)
    .bind(holding.amount_fen)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|_| errors::internal_error())?;
    sqlx::query("UPDATE accounts SET available_balance_fen = available_balance_fen + $1 WHERE id = $2")
        .bind(holding.amount_fen)
        .bind(&account.id)
        .execute(&mut *tx)
        .await
        .map_err(|_| errors::internal_error())?;
    tx.commit()
        .await
        .map_err(|_| errors::internal_error())?;
    audit::append_event(
        &state.pool,
        AppendEventParams {
            action: "wealth.redeem",
            actor_user_id: Some(&user.id),
            session: None,
            request_id: Some(&rid.0),
            workflow_id: None,
            node_id: None,
            tool: Some("wealth.redeem"),
            parameters_masked: Some(json!({ "holding_id": holding.id, "amount_fen": holding.amount_fen })),
            risk_tier: Some(policy::decide_wealth_redeem().tier.as_str()),
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
        "holding_id": holding.id,
        "status": "redeemed",
        "amount_fen": holding.amount_fen,
    })))
}

// silence unused
#[allow(dead_code)]
async fn _account_silence() -> Account {
    Account {
        id: String::new(),
        user_id: String::new(),
        display_name: String::new(),
        masked_number: String::new(),
        currency: String::new(),
        available_balance_fen: 0,
        reserved_balance_fen: 0,
        created_at: Utc::now(),
    }
}
