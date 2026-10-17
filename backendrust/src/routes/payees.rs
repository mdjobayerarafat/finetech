//! Payees and transfers routes.

use actix_web::{web, HttpResponse};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;

use crate::audit::{self, AppendEventParams};
use crate::ctx::{require_csrf, AuthUser, RequestId};
use crate::errors::{self, ApiError};
use crate::ids;
use crate::models::{Account, Payee, Transfer, User};
use crate::money::coerce_amount_fen;
use crate::planner;
use crate::policy;
use crate::safety;
use crate::tools::ToolError;

use crate::state::AppState;
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/payees", web::get().to(list_payees))
        .route("/payees", web::post().to(create_payee))
        .route("/transfers/preview", web::post().to(transfer_preview))
        .route("/transfers", web::post().to(create_transfer))
        .route("/transfers", web::get().to(list_transfers))
        .route("/transfers/{id}/cancel", web::post().to(cancel_transfer));
}

async fn list_payees(
    state: web::Data<AppState>,
    auth: AuthUser,
) -> Result<HttpResponse, ApiError> {
    let user = auth.0;
    let rows: Vec<Payee> = sqlx::query_as(
        "SELECT * FROM payees WHERE owner_user_id = $1 ORDER BY display_name ASC",
    )
    .bind(&user.id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    let out: Vec<_> = rows
        .into_iter()
        .map(|p| {
            json!({
                "id": p.id,
                "display_name": p.display_name,
                "phone_tail": p.phone_tail,
                "mock_destination": p.mock_destination,
                "verified": p.verified,
                "first_used_at": p.first_used_at,
            })
        })
        .collect();
    Ok(HttpResponse::Ok().json(out))
}

#[derive(Deserialize)]
pub struct PayeeIn {
    pub display_name: String,
    pub phone_tail: Option<String>,
    #[serde(default = "default_destination")]
    pub mock_destination: String,
}
fn default_destination() -> String {
    "****demo".to_string()
}

async fn create_payee(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    body: web::Json<PayeeIn>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let user = auth.0;
    let body = body.into_inner();
    let id = ids::new_id("payee");
    sqlx::query(
        "INSERT INTO payees (id, owner_user_id, display_name, phone_tail, mock_destination, verified, created_at) VALUES ($1, $2, $3, $4, $5, 0, $6)",
    )
    .bind(&id)
    .bind(&user.id)
    .bind(&body.display_name)
    .bind(&body.phone_tail)
    .bind(&body.mock_destination)
    .bind(Utc::now())
    .execute(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    audit::append_event(
        &state.pool,
        AppendEventParams {
            action: "payee.create",
            actor_user_id: Some(&user.id),
            session: None,
            request_id: Some(&rid.0),
            workflow_id: None,
            node_id: None,
            tool: None,
            parameters_masked: Some(json!({ "payee_id": id, "display_name": body.display_name })),
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
    let row: Payee = sqlx::query_as("SELECT * FROM payees WHERE id = $1")
        .bind(&id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    Ok(HttpResponse::Ok().json(json!({
        "id": row.id,
        "display_name": row.display_name,
        "phone_tail": row.phone_tail,
        "mock_destination": row.mock_destination,
        "verified": row.verified,
        "first_used_at": row.first_used_at,
    })))
}

#[derive(Deserialize)]
pub struct TransferPreviewIn {
    pub account_id: Option<String>,
    pub recipient_query: Option<String>,
    pub payee_id: Option<String>,
    pub phone_tail: Option<String>,
    pub amount: serde_json::Value,
    #[serde(default)]
    pub memo: String,
    pub schedule: Option<DateTime<Utc>>,
}

async fn transfer_preview(
    state: web::Data<AppState>,
    auth: AuthUser,
    body: web::Json<TransferPreviewIn>,
) -> Result<HttpResponse, ApiError> {
    let user = auth.0;
    let body = body.into_inner();
    let account = match planner::resolve_account(
        &state.pool,
        &user,
        body.account_id.as_deref(),
    )
    .await
    {
        Ok(a) => a,
        Err(_) => return Err(errors::not_found("Account not found.")),
    };
    let amount_fen = coerce_amount_fen(&body.amount).map_err(|e| errors::schema_invalid(&e))?;
    let payee_result = if let Some(t) = body.phone_tail.clone() {
        planner::resolve_payee_by_phone_tail(&state.pool, &user, &t).await
    } else {
        let q = body
            .recipient_query
            .clone()
            .or(body.payee_id.clone())
            .unwrap_or_default();
        planner::resolve_payee_by_query(&state.pool, &user, &q).await
    };
    let payee = match payee_result {
        Ok(p) => p,
        Err(ToolError::Clarification(m)) => {
            return Err(errors::clarification_required(vec![m]));
        }
        Err(ToolError::NotFound(m)) => return Err(errors::not_found(m)),
        Err(e) => return Err(errors::schema_invalid(&e.to_string())),
    };
    let decision = policy::decide_transfer(
        &state.pool,
        &user,
        &account,
        &payee,
        amount_fen,
        &body.memo,
    )
    .await
    .map_err(|_| errors::internal_error())?;
    let settings = crate::config::get_settings();
    let today = policy::today_total_fen(&state.pool, &account.id)
        .await
        .map_err(|_| errors::internal_error())?;
    let remaining = std::cmp::max(0, settings.daily_limit_fen - today - amount_fen);
    Ok(HttpResponse::Ok().json(json!({
        "resolved_payee_id": payee.id,
        "resolved_payee_name": payee.display_name,
        "resolved_payee_destination": payee.mock_destination,
        "amount_fen": amount_fen,
        "risk_tier": decision.tier,
        "decision_reason": decision.reason,
        "daily_remaining_fen": remaining,
    })))
}

async fn create_transfer(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    body: web::Json<TransferPreviewIn>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let user = auth.0;
    let body = body.into_inner();
    safety::assert_unlocked(&user)?;
    let amount_fen = coerce_amount_fen(&body.amount).map_err(|e| errors::schema_invalid(&e))?;
    let mut args = serde_json::Map::new();
    if let Some(a) = body.account_id {
        args.insert("account_id".into(), serde_json::Value::String(a));
    }
    if let Some(r) = body.recipient_query {
        args.insert("recipient_query".into(), serde_json::Value::String(r));
    }
    if let Some(p) = body.payee_id {
        args.insert("payee_id".into(), serde_json::Value::String(p));
    }
    if let Some(t) = body.phone_tail {
        args.insert("phone_tail".into(), serde_json::Value::String(t));
    }
    args.insert("amount_fen".into(), serde_json::Value::Number(amount_fen.into()));
    args.insert("memo".into(), serde_json::Value::String(body.memo));
    if let Some(s) = body.schedule {
        args.insert("schedule".into(), serde_json::Value::String(s.to_rfc3339()));
    }
    let args_val = serde_json::Value::Object(args);
    let action = json!({
        "tool": "transfer.create_pending",
        "arguments": args_val,
    });
    let workflow = match planner::build_plan(&state.pool, &user, &[action], &rid.0).await {
        Ok(w) => w,
        Err(ToolError::NotFound(m)) => return Err(errors::not_found(m)),
        Err(ToolError::InsufficientFunds(m)) => {
            return Err(errors::ApiError::new("insufficient_funds", m, 422));
        }
        Err(ToolError::Clarification(m)) => {
            return Err(errors::clarification_required(vec![m]));
        }
        Err(e) => return Err(errors::schema_invalid(&e.to_string())),
    };
    let nodes: Vec<crate::models::WorkflowNode> = sqlx::query_as(
        "SELECT * FROM workflow_nodes WHERE workflow_id = $1 ORDER BY node_index ASC",
    )
    .bind(&workflow.id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    let first = nodes.first();
    Ok(HttpResponse::Ok().json(json!({
        "transfer_id": ids::new_id("xfer"),
        "pending_action_id": first.and_then(|n| n.pending_action_id.clone()),
        "risk_tier": first.map(|n| n.risk_tier.clone()).unwrap_or_else(|| "Green".to_string()),
    })))
}

async fn cancel_transfer(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let user = auth.0;
    let id = path.into_inner();
    let row: Option<Transfer> = sqlx::query_as("SELECT * FROM transfers WHERE id = $1")
        .bind(&id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    let row = row.ok_or_else(|| errors::not_found("Transfer not found."))?;
    let account: Option<Account> = sqlx::query_as("SELECT * FROM accounts WHERE id = $1")
        .bind(&row.source_account_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    let account = account.ok_or_else(|| errors::not_found("Transfer not found."))?;
    if account.user_id != user.id {
        return Err(errors::not_found("Transfer not found."));
    }
    if matches!(row.status.as_str(), "posted" | "reversed" | "failed") {
        return Err(errors::ApiError::new(
            "schema_invalid",
            &format!("Cannot cancel a transfer in status '{}'.", row.status),
            409,
        ));
    }
    sqlx::query("UPDATE transfers SET status = 'reversed' WHERE id = $1")
        .bind(&id)
        .execute(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    audit::append_event(
        &state.pool,
        AppendEventParams {
            action: "transfer.cancel",
            actor_user_id: Some(&user.id),
            session: None,
            request_id: Some(&rid.0),
            workflow_id: None,
            node_id: None,
            tool: Some("transfer.cancel"),
            parameters_masked: Some(json!({ "transfer_id": id })),
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
        "transfer_id": id,
        "status": "reversed",
    })))
}

#[derive(Deserialize)]
pub struct ListTransfersQuery {
    pub status: Option<String>,
    #[serde(default = "default_tlimit")]
    pub limit: i64,
}
fn default_tlimit() -> i64 {
    50
}

async fn list_transfers(
    state: web::Data<AppState>,
    auth: AuthUser,
    q: web::Query<ListTransfersQuery>,
) -> Result<HttpResponse, ApiError> {
    let user = auth.0;
    let q = q.into_inner();
    let mut sql = String::from(
        "SELECT t.* FROM transfers t JOIN accounts a ON a.id = t.source_account_id WHERE a.user_id = ?",
    );
    if q.status.is_some() {
        sql.push_str(" AND t.status = ?");
    }
    sql.push_str(" ORDER BY t.created_at DESC LIMIT ?");
    let mut query = sqlx::query_as::<_, Transfer>(&sql).bind(&user.id);
    if let Some(s) = q.status {
        query = query.bind(s);
    }
    query = query.bind(q.limit);
    let rows = query
        .fetch_all(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    let out: Vec<_> = rows
        .into_iter()
        .map(|r| {
            json!({
                "id": r.id,
                "source_account_id": r.source_account_id,
                "payee_id": r.payee_id,
                "amount_fen": r.amount_fen,
                "memo": r.memo,
                "schedule": r.schedule,
                "status": r.status,
                "created_at": r.created_at,
                "posted_at": r.posted_at,
            })
        })
        .collect();
    Ok(HttpResponse::Ok().json(out))
}
