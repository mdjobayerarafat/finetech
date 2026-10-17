//! Per-category monthly budget routes.
//!
//! The agent (`/api/v1/agent/...`) routes its `budgets.set` / `budgets.reserve`
//! tool calls through `planner::build_plan` → the same risk/MFA pipeline as
//! transfers. These direct REST routes exist for the dashboard / cost-
//! management page to set caps without going through chat, and for the
//! forecast + optimization reports to read them.

use actix_web::{web, HttpResponse};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;

use crate::ctx::AuthUser;
use crate::errors::{self, ApiError};
use crate::ids;
use crate::models::Budget;
use crate::money::{coerce_amount_fen, fen_to_cny_str};
use crate::safety;
use crate::state::AppState;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/budgets", web::get().to(list_budgets))
        .route("/budgets", web::post().to(set_budget))
        .route("/budgets/{id}", web::delete().to(delete_budget))
        .route("/budgets/reservations", web::get().to(list_reservations))
        .route("/budgets/summary", web::get().to(summary));
}

#[derive(Deserialize)]
pub struct SetBudgetIn {
    pub category: String,
    pub amount: serde_json::Value, // string ("3000") or number — same coercion as transfers
    #[serde(default)]
    pub account_id: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

async fn set_budget(
    state: web::Data<AppState>,
    auth: AuthUser,
    body: web::Json<SetBudgetIn>,
) -> Result<HttpResponse, ApiError> {
    safety::assert_unlocked(&auth.0)?;
    let body = body.into_inner();
    let limit_fen = coerce_amount_fen(&body.amount)
        .map_err(|e| errors::schema_invalid(&e))?;
    if limit_fen <= 0 {
        return Err(errors::schema_invalid("amount must be > 0"));
    }
    let user_id = auth.0.id.clone();

    // Idempotent on (user, category, account) — see t_budget_set.
    let existing: Option<Budget> = sqlx::query_as(
        "SELECT * FROM budgets WHERE user_id = $1 AND category = $2 \
         AND ((account_id IS NULL AND $3::text IS NULL) OR account_id = $3) \
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(&user_id)
    .bind(&body.category)
    .bind(body.account_id.as_deref())
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;

    let budget = if let Some(mut b) = existing {
        sqlx::query(
            "UPDATE budgets SET limit_fen = $2, note = $3, updated_at = NOW() WHERE id = $1",
        )
        .bind(&b.id)
        .bind(limit_fen)
        .bind(body.note.as_deref())
        .execute(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
        b.limit_fen = limit_fen;
        b.note = body.note.clone();
        b.updated_at = Utc::now();
        b
    } else {
        let id = ids::new_id("bgt");
        let now: DateTime<Utc> = Utc::now();
        sqlx::query(
            "INSERT INTO budgets (id, user_id, category, limit_fen, account_id, note, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $7)",
        )
        .bind(&id)
        .bind(&user_id)
        .bind(&body.category)
        .bind(limit_fen)
        .bind(body.account_id.as_deref())
        .bind(body.note.as_deref())
        .bind(now)
        .execute(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
        Budget {
            id,
            user_id: user_id.clone(),
            category: body.category,
            limit_fen,
            period_start: None,
            period_end: None,
            account_id: body.account_id,
            note: body.note,
            created_at: now,
            updated_at: now,
        }
    };

    Ok(HttpResponse::Ok().json(json!({
        "id": budget.id,
        "category": budget.category,
        "limit_fen": budget.limit_fen,
        "account_id": budget.account_id,
        "note": budget.note,
        "created_at": budget.created_at,
        "updated_at": budget.updated_at,
    })))
}

async fn list_budgets(
    state: web::Data<AppState>,
    auth: AuthUser,
) -> Result<HttpResponse, ApiError> {
    let user_id = auth.0.id;
    let rows: Vec<Budget> = sqlx::query_as(
        "SELECT * FROM budgets WHERE user_id = $1 ORDER BY category ASC, created_at DESC",
    )
    .bind(&user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    let mut out: Vec<serde_json::Value> = Vec::with_capacity(rows.len());
    for b in &rows {
        let spend = spend_for_budget(&state.pool, &user_id, &b.category, b.account_id.as_deref())
            .await
            .unwrap_or(0);
        let pct = if b.limit_fen > 0 {
            (spend as f64 / b.limit_fen as f64) * 100.0
        } else {
            0.0
        };
        out.push(json!({
            "id": b.id,
            "category": b.category,
            "limit_fen": b.limit_fen,
            "spent_fen": spend,
            "remaining_fen": (b.limit_fen - spend).max(0),
            "utilization_pct": pct,
            "breached": spend > b.limit_fen,
            "account_id": b.account_id,
            "note": b.note,
            "created_at": b.created_at,
            "updated_at": b.updated_at,
        }));
    }
    Ok(HttpResponse::Ok().json(json!({ "budgets": out })))
}

async fn delete_budget(
    state: web::Data<AppState>,
    auth: AuthUser,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let id = path.into_inner();
    let res = sqlx::query("DELETE FROM budgets WHERE id = $1 AND user_id = $2")
        .bind(&id)
        .bind(&auth.0.id)
        .execute(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    if res.rows_affected() == 0 {
        return Err(errors::not_found("Budget not found"));
    }
    Ok(HttpResponse::Ok().json(json!({ "deleted": true, "id": id })))
}

async fn list_reservations(
    state: web::Data<AppState>,
    auth: AuthUser,
) -> Result<HttpResponse, ApiError> {
    let rows = sqlx::query_as::<_, crate::models::BudgetReservation>(
        "SELECT * FROM budget_reservations WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(&auth.0.id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    let total: i64 = rows.iter().map(|r| r.amount_fen).sum();
    Ok(HttpResponse::Ok().json(json!({
        "reservations": rows,
        "total_held_fen": total,
    })))
}

/// Aggregate summary used by the dashboard / cost-management / chat agent.
async fn summary(
    state: web::Data<AppState>,
    auth: AuthUser,
) -> Result<HttpResponse, ApiError> {
    let user_id = auth.0.id;
    let rows: Vec<Budget> = sqlx::query_as(
        "SELECT * FROM budgets WHERE user_id = $1",
    )
    .bind(&user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;

    let mut total_limit: i64 = 0;
    let mut total_spent: i64 = 0;
    let mut breached: Vec<serde_json::Value> = Vec::new();
    let mut near_limit: Vec<serde_json::Value> = Vec::new();

    for b in &rows {
        let spend = spend_for_budget(&state.pool, &user_id, &b.category, b.account_id.as_deref())
            .await
            .unwrap_or(0);
        total_limit += b.limit_fen;
        total_spent += spend;
        let pct = if b.limit_fen > 0 {
            (spend as f64 / b.limit_fen as f64) * 100.0
        } else {
            0.0
        };
        let row = json!({
            "category": b.category,
            "limit_fen": b.limit_fen,
            "spent_fen": spend,
            "utilization_pct": pct,
        });
        if spend > b.limit_fen {
            breached.push(row);
        } else if pct >= 80.0 {
            near_limit.push(row);
        }
    }

    let held_res: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount_fen), 0)::BIGINT FROM budget_reservations WHERE user_id = $1 AND state = 'held'",
    )
    .bind(&user_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;

    Ok(HttpResponse::Ok().json(json!({
        "budget_count": rows.len(),
        "total_limit_fen": total_limit,
        "total_spent_fen": total_spent,
        "remaining_fen": (total_limit - total_spent).max(0),
        "breached": breached,
        "near_limit": near_limit,
        "held_reservations_fen": held_res,
        "display": {
            "limit": fen_to_cny_str(total_limit),
            "spent": fen_to_cny_str(total_spent),
            "remaining": fen_to_cny_str((total_limit - total_spent).max(0)),
        }
    })))
}

/// Helper — sum of debits for a (user, category[, account]) in the last 30
/// days, matching `/reports/spending`'s window so the dashboard and the
/// agent never disagree.
async fn spend_for_budget(
    pool: &PgPool,
    user_id: &str,
    category: &str,
    account_id: Option<&str>,
) -> Result<i64, sqlx::Error> {
    if let Some(acc) = account_id {
        sqlx::query_scalar(
            "SELECT COALESCE(SUM(-t.amount_fen), 0)::BIGINT FROM transactions t \
             JOIN accounts a ON a.id = t.account_id \
             WHERE a.user_id = $1 AND a.id = $2 AND t.category = $3 \
             AND t.amount_fen < 0 AND t.occurred_at >= NOW() - INTERVAL '30 days'",
        )
        .bind(user_id)
        .bind(acc)
        .bind(category)
        .fetch_one(pool)
        .await
    } else {
        sqlx::query_scalar(
            "SELECT COALESCE(SUM(-t.amount_fen), 0)::BIGINT FROM transactions t \
             JOIN accounts a ON a.id = t.account_id \
             WHERE a.user_id = $1 AND t.category = $2 \
             AND t.amount_fen < 0 AND t.occurred_at >= NOW() - INTERVAL '30 days'",
        )
        .bind(user_id)
        .bind(category)
        .fetch_one(pool)
        .await
    }
}
