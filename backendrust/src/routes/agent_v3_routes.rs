//! Routes for the agent v3 tools (Layer 1 close-the-loop, 2.1 plan persistence,
//! 3.1 daily digest, 3.4 goal coaching).
//!
//! These HTTP handlers are thin shims — they call the canonical tool functions
//! in `agent_v3` and pass through the typed result. They exist so the frontend
//! can hit the new endpoints directly without going through the chat agent.

use actix_web::{web, HttpResponse};
use chrono::{Duration, Utc};
use rand::Rng;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::audit::{self, AppendEventParams};
use crate::ctx::{require_csrf, AuthUser, RequestId};
use crate::errors::{self, ApiError};
use crate::ids;
use crate::models::{PendingAction, SavingsGoal};
use crate::safety;
use crate::tools::{self, ToolContext, ToolError};

use crate::state::AppState;

pub fn configure(cfg: &mut web::ServiceConfig) {
    // Transfers
    cfg.route("/transfers/{id}/status", web::get().to(transfers_get_status))
        .route("/transfers/{id}/reverse", web::post().to(transfers_reverse));
    // Subscriptions
    cfg.route("/subscriptions/{id}/snooze", web::post().to(subscriptions_snooze))
        .route(
            "/subscriptions/{id}/defer",
            web::post().to(subscriptions_defer),
        )
        .route(
            "/subscriptions/{id}/uncancel",
            web::post().to(subscriptions_uncancel),
        );
    // Transactions
    cfg.route("/transactions/search", web::get().to(transactions_search))
        .route(
            "/transactions/merchant/{name}/history",
            web::get().to(transactions_merchant_history),
        )
        .route(
            "/transactions/{id}/dispute-draft",
            web::post().to(transactions_dispute_draft),
        );
    // Payees (extend existing module too)
    cfg.route("/payees/merge-duplicates", web::post().to(payees_merge_duplicates))
        .route("/payees/{id}/verify", web::post().to(payees_verify_request));
    // Workflows
    cfg.route("/workflows-pending", web::get().to(workflows_list_pending));
    // Actions
    cfg.route("/actions/{id}/defer", web::post().to(actions_defer));
    // Insights + goals
    cfg.route("/insights/daily", web::get().to(insights_daily))
        .route("/insights/daily/ack", web::post().to(insights_daily_ack))
        .route("/goals/{id}/progress", web::get().to(goals_progress))
        .route("/goals/{id}/rebalance", web::post().to(goals_rebalance))
        .route("/goals/{id}/contribute", web::post().to(goals_contribute));
    // List the caller's savings goals so the wealth page can render a
    // progress strip without round-tripping per goal.
    cfg.route("/goals", web::get().to(goals_list));
}

// GET /api/v1/goals — list the caller's savings goals with a tiny
// contribution summary (lifetime saved, this month, progress %, ETA in
// days). Computed inline so the UI does not need a second round-trip per
// goal.
async fn goals_list(
    state: web::Data<AppState>,
    auth: AuthUser,
) -> Result<HttpResponse, ApiError> {
    let user = auth.0;
    let rows: Vec<SavingsGoal> = sqlx::query_as(
        "SELECT * FROM savings_goals WHERE user_id = $1 ORDER BY created_at ASC",
    )
    .bind(&user.id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    let mut items = Vec::with_capacity(rows.len());
    for g in rows {
        let total: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(amount_fen), 0) FROM goal_contributions WHERE goal_id = $1",
        )
        .bind(&g.id)
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);
        let this_month: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(amount_fen), 0) FROM goal_contributions \
             WHERE goal_id = $1 AND created_at >= date_trunc('month', NOW())",
        )
        .bind(&g.id)
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);
        let pct: i64 = if g.target_fen > 0 {
            (total * 100) / g.target_fen
        } else {
            0
        };
        // Naive ETA: assume this month's pace continues. 30-day projection
        // mirrors the per-goal progress tool, so the panel and the per-row
        // drilldown agree.
        let daily_pace: i64 = if this_month > 0 { this_month / 30 } else { 0 };
        let remaining = (g.target_fen - total).max(0);
        let days_to_goal: Option<i64> = if daily_pace > 0 {
            Some(remaining / daily_pace)
        } else {
            None
        };
        items.push(json!({
            "id": g.id,
            "name": g.name,
            "target_fen": g.target_fen,
            "saved_fen": total,
            "this_month_fen": this_month,
            "progress_pct": pct,
            "daily_pace_fen": daily_pace,
            "days_to_goal": days_to_goal,
            "created_at": g.created_at,
        }));
    }
    Ok(HttpResponse::Ok().json(json!({
        "items": items,
        "count": items.len(),
    })))
}

// --- Shared helper: invoke a tool with the right context --------------------

async fn invoke_tool(
    state: &web::Data<AppState>,
    auth: &AuthUser,
    rid: &RequestId,
    name: &'static str,
    args: Value,
) -> Result<HttpResponse, ApiError> {
    let ctx = ToolContext {
        pool: state.pool.clone(),
        user: auth.0.clone(),
        request_id: rid.0.clone(),
        session_id: None,
    };
    let f = tools::registry()
        .get(name)
        .copied()
        .ok_or_else(|| errors::ApiError::new("not_found", &format!("Tool {} not found.", name), 500))?;
    match f(&ctx, &args).await {
        Ok(v) => Ok(HttpResponse::Ok().json(v)),
        Err(e) => Err(tool_err_to_api(e)),
    }
}

fn tool_err_to_api(e: ToolError) -> ApiError {
    let code = e.code();
    let status = match e {
        ToolError::Schema(_) => 400,
        ToolError::NotFound(_) => 404,
        ToolError::InsufficientFunds(_) => 422,
        ToolError::BelowMinimum(_) => 422,
        ToolError::Clarification(_) => 400,
        ToolError::Forbidden(_) => 403,
        ToolError::Other(_) => 500,
    };
    // Layer 5.5 — friendlier messages than the raw tool toString for
    // the common cases. The "What to do next" hint comes through `details`.
    let (message, details) = friendly_error(&e);
    errors::ApiError::new(code, &message, status).with_details_or(details)
}

fn friendly_error(e: &ToolError) -> (String, Option<Value>) {
    match e {
        ToolError::Schema(msg) => {
            // Heuristic: if the message already says what to do, leave it.
            // Otherwise wrap with a "what next" hint.
            if msg.contains("5-minute") || msg.contains("24 hours") || msg.contains("already") {
                (msg.to_string(), None)
            } else {
                (
                    msg.to_string(),
                    Some(json!({ "what_next": "Check the input and try again, or ask the agent to help." })),
                )
            }
        }
        ToolError::NotFound(m) => (m.to_string(), None),
        ToolError::InsufficientFunds(m) => (
            m.to_string(),
            Some(json!({ "what_next": "Top up the account, pick a different source, or reduce the amount." })),
        ),
        ToolError::BelowMinimum(m) => (
            m.to_string(),
            Some(json!({ "what_next": "Increase the amount, or pick a different goal." })),
        ),
        ToolError::Clarification(m) => (
            "The agent needs more information to continue.".to_string(),
            Some(json!({ "question": m })),
        ),
        ToolError::Forbidden(m) => (
            m.to_string(),
            Some(json!({ "what_next": "An admin rule blocks this. Open the chat to ask for an override." })),
        ),
        ToolError::Other(m) => (m.to_string(), None),
    }
}

trait WithDetailsOr {
    fn with_details_or(self, details: Option<Value>) -> Self;
}
impl WithDetailsOr for ApiError {
    fn with_details_or(mut self, details: Option<Value>) -> Self {
        if let Some(d) = details {
            self.details = Some(d);
        }
        self
    }
}

// --- Transfers --------------------------------------------------------------

async fn transfers_get_status(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let id = path.into_inner();
    invoke_tool(&state, &auth, &rid, "transfers.get_status", json!({ "transfer_id": id })).await
}

async fn transfers_reverse(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    body: web::Json<ReverseIn>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let id = path.into_inner();
    let body = body.into_inner();
    // Red tier — write a pending_action so the user must confirm (and MFA if
    // their account requires it). Then we surface the standard 403/409
    // envelope from the actions layer rather than executing inline.
    let user = auth.0.clone();
    safety::assert_unlocked(&user)?;
    let settings = crate::config::get_settings();
    let tool_args = json!({ "transfer_id": id, "reason": body.reason });
    let parameter_hash = crate::canonical::parameter_hash(&tool_args);
    let pa_id = ids::new_id("pa");
    let now = Utc::now();
    let expires = now + Duration::minutes(5);
    sqlx::query(
        "INSERT INTO pending_actions (id, user_id, tool, parameters, parameter_hash, risk_tier, mfa_status, confirmation_status, expires_at, created_at) \
         VALUES (?, ?, 'transfers.reverse_recent', ?, ?, 'Red', 'none', 'none', ?, ?)",
    )
    .bind(&pa_id)
    .bind(&user.id)
    .bind(tool_args.to_string())
    .bind(&parameter_hash)
    .bind(expires)
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    audit::append_event(
        &state.pool,
        AppendEventParams {
            action: "transfer.reverse.create",
            actor_user_id: Some(&user.id),
            session: None,
            request_id: Some(&rid.0),
            workflow_id: None,
            node_id: None,
            tool: Some("transfers.reverse_recent"),
            parameters_masked: Some(json!({ "transfer_id": id, "reason": body.reason })),
            risk_tier: Some("Red"),
            policy_reason: None,
            confirmation_status: Some("none"),
            mfa_status: Some("none"),
            result: "ok",
            error_code: None,
        },
    )
    .await
    .ok();
    let _ = settings;
    let pa: PendingAction = sqlx::query_as("SELECT * FROM pending_actions WHERE id = ?")
        .bind(&pa_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    Ok(HttpResponse::Ok().json(json!({
        "pending_action_id": pa.id,
        "risk_tier": pa.risk_tier,
        "expires_at": pa.expires_at,
        "next_step": "verify with mock MFA, then confirm to execute",
        "mock_mfa_code": settings.mfa_demo_code,
    })))
}

#[derive(Deserialize)]
pub struct ReverseIn {
    pub reason: String,
}

// --- Subscriptions ----------------------------------------------------------

#[derive(Deserialize)]
pub struct SnoozeIn {
    pub days: i64,
}

async fn subscriptions_snooze(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    body: web::Json<SnoozeIn>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let id = path.into_inner();
    let body = body.into_inner();
    invoke_tool(
        &state,
        &auth,
        &rid,
        "subscriptions.snooze",
        json!({ "subscription_id": id, "days": body.days }),
    )
    .await
}

#[derive(Deserialize)]
pub struct DeferIn {
    #[serde(default)]
    pub reason: String,
}

async fn subscriptions_defer(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    body: web::Json<DeferIn>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let id = path.into_inner();
    let body = body.into_inner();
    invoke_tool(
        &state,
        &auth,
        &rid,
        "subscriptions.defer_charge",
        json!({ "subscription_id": id, "reason": body.reason }),
    )
    .await
}

async fn subscriptions_uncancel(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let id = path.into_inner();
    invoke_tool(
        &state,
        &auth,
        &rid,
        "subscriptions.uncancel_recent",
        json!({ "subscription_id": id }),
    )
    .await
}

// --- Transactions -----------------------------------------------------------

#[derive(Deserialize)]
pub struct SearchQuery {
    pub query: String,
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default)]
    pub min_amount_fen: Option<i64>,
    #[serde(default)]
    pub max_amount_fen: Option<i64>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub limit: Option<i64>,
}

async fn transactions_search(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    q: web::Query<SearchQuery>,
) -> Result<HttpResponse, ApiError> {
    let q = q.into_inner();
    let mut args = json!({ "query": q.query });
    if let Some(v) = q.from {
        args["from"] = json!(v);
    }
    if let Some(v) = q.to {
        args["to"] = json!(v);
    }
    if let Some(v) = q.min_amount_fen {
        args["min_amount_fen"] = json!(v);
    }
    if let Some(v) = q.max_amount_fen {
        args["max_amount_fen"] = json!(v);
    }
    if let Some(v) = q.category {
        args["category"] = json!(v);
    }
    if let Some(v) = q.limit {
        args["limit"] = json!(v);
    }
    invoke_tool(&state, &auth, &rid, "transactions.search", args).await
}

async fn transactions_merchant_history(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let name = path.into_inner();
    invoke_tool(
        &state,
        &auth,
        &rid,
        "transactions.merchant_history",
        json!({ "merchant": name }),
    )
    .await
}

#[derive(Deserialize)]
pub struct DisputeIn {
    pub reason: String,
}

async fn transactions_dispute_draft(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    body: web::Json<DisputeIn>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let id = path.into_inner();
    let body = body.into_inner();
    invoke_tool(
        &state,
        &auth,
        &rid,
        "transactions.dispute_draft",
        json!({ "transaction_id": id, "reason": body.reason }),
    )
    .await
}

// --- Payees -----------------------------------------------------------------

#[derive(Deserialize)]
pub struct MergeIn {
    pub keep_id: String,
    pub merge_id: String,
}

async fn payees_merge_duplicates(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    body: web::Json<MergeIn>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let body = body.into_inner();
    invoke_tool(
        &state,
        &auth,
        &rid,
        "payees.merge_duplicates",
        json!({ "keep_id": body.keep_id, "merge_id": body.merge_id }),
    )
    .await
}

#[derive(Deserialize)]
pub struct VerifyReqIn {
    pub challenge_fen: i64,
}

async fn payees_verify_request(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    body: web::Json<VerifyReqIn>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let user = auth.0.clone();
    let payee_id = path.into_inner();
    // Confirm the payee belongs to this user.
    let owner: Option<String> = sqlx::query_scalar("SELECT owner_user_id FROM payees WHERE id = ?")
        .bind(&payee_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    let owner = owner.ok_or_else(|| errors::not_found("Payee not found."))?;
    if owner != user.id {
        return Err(errors::not_found("Payee not found."));
    }
    // Generate a 4-digit challenge code.
    let mut r = rand::thread_rng();
    let code = format!("{:04}", r.gen_range(0..10000));
    let ver_id = ids::new_id("pver");
    let now = Utc::now();
    let expires = now + Duration::minutes(10);
    sqlx::query(
        "INSERT INTO payee_verifications (id, payee_id, user_id, challenge_fen, code, status, expires_at, created_at) VALUES (?, ?, ?, ?, ?, 'pending', ?, ?)",
    )
    .bind(&ver_id)
    .bind(&payee_id)
    .bind(&user.id)
    .bind(body.challenge_fen)
    .bind(&code)
    .bind(expires)
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    sqlx::query("UPDATE payees SET verification_status = 'pending' WHERE id = ?")
        .bind(&payee_id)
        .execute(&state.pool)
        .await
        .ok();
    audit::append_event(
        &state.pool,
        AppendEventParams {
            action: "payee.verify.start",
            actor_user_id: Some(&user.id),
            session: None,
            request_id: Some(&rid.0),
            workflow_id: None,
            node_id: None,
            tool: Some("payees.verify"),
            parameters_masked: Some(json!({ "payee_id": payee_id, "challenge_fen": body.challenge_fen })),
            risk_tier: Some("Yellow"),
            policy_reason: None,
            confirmation_status: None,
            mfa_status: None,
            result: "ok",
            error_code: None,
        },
    )
    .await
    .ok();
    // Surface the demo code in the response so the user can finish the round-trip.
    // (In a real product this would be a push notification or a one-tap
    // confirmation; for the simulation we just print it.)
    Ok(HttpResponse::Ok().json(json!({
        "verification_id": ver_id,
        "payee_id": payee_id,
        "challenge_fen": body.challenge_fen,
        "expires_at": expires,
        "demo_code": code,
        "next_step": "Send the small challenge amount to the payee, then POST /api/v1/payees/{id}/verify-confirm with the code."
    })))
}

// --- Workflows + actions ----------------------------------------------------

async fn workflows_list_pending(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
) -> Result<HttpResponse, ApiError> {
    invoke_tool(&state, &auth, &rid, "workflows.list_pending", json!({})).await
}

async fn workflows_resume(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let id = path.into_inner();
    invoke_tool(
        &state,
        &auth,
        &rid,
        "workflows.resume",
        json!({ "workflow_id": id }),
    )
    .await
}

#[derive(Deserialize)]
pub struct DeferActionIn {
    pub minutes: i64,
}

async fn actions_defer(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    body: web::Json<DeferActionIn>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let id = path.into_inner();
    let body = body.into_inner();
    invoke_tool(
        &state,
        &auth,
        &rid,
        "actions.defer",
        json!({ "pending_action_id": id, "minutes": body.minutes }),
    )
    .await
}

// --- Insights + goals -------------------------------------------------------

async fn insights_daily(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
) -> Result<HttpResponse, ApiError> {
    invoke_tool(&state, &auth, &rid, "insights.daily_digest", json!({ "ack": false })).await
}

async fn insights_daily_ack(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    invoke_tool(&state, &auth, &rid, "insights.daily_digest", json!({ "ack": true })).await
}

async fn goals_progress(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let id = path.into_inner();
    invoke_tool(
        &state,
        &auth,
        &rid,
        "goals.progress",
        json!({ "goal_id": id }),
    )
    .await
}

#[derive(Deserialize)]
pub struct RebalanceIn {
    pub monthly_amount_fen: i64,
    #[serde(default)]
    pub source_account_id: Option<String>,
}

async fn goals_rebalance(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    body: web::Json<RebalanceIn>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let id = path.into_inner();
    let body = body.into_inner();
    let mut args = json!({ "goal_id": id, "monthly_amount_fen": body.monthly_amount_fen });
    if let Some(s) = body.source_account_id {
        args["source_account_id"] = json!(s);
    }
    invoke_tool(&state, &auth, &rid, "goals.rebalance", args).await
}

#[derive(Deserialize)]
pub struct ContributeIn {
    pub amount_fen: i64,
    pub source_account_id: String,
    #[serde(default)]
    pub memo: String,
}

async fn goals_contribute(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    body: web::Json<ContributeIn>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let id = path.into_inner();
    let body = body.into_inner();
    invoke_tool(
        &state,
        &auth,
        &rid,
        "goals.contribute",
        json!({
            "goal_id": id,
            "amount_fen": body.amount_fen,
            "source_account_id": body.source_account_id,
            "memo": body.memo,
        }),
    )
    .await
}
