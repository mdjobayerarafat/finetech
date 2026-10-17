//! Audit + admin routes.

use actix_web::{web, HttpResponse};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::json;

use crate::agent_rules::{
    self, NewAgentRule, UpdateAgentRule,
};
use crate::audit;
use crate::conversations::{Conversation, ConversationMessage};
use crate::ctx::{require_admin, require_csrf, AuthUser, RequestId};
use crate::errors::{self, ApiError};
use crate::models::{Account, AuditEvent, SafetyEvent, Transaction, User};
use crate::safety;
use crate::state::AppState;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/audit", web::get().to(list_audit))
        .route("/audit/verify-chain", web::get().to(verify_chain))
        .route("/admin/safety-events", web::get().to(list_safety_events))
        .route(
            "/admin/safety-lock/{id}/unlock",
            web::post().to(admin_unlock),
        )
        .route("/admin/reset", web::post().to(admin_reset))
        .route("/admin/users", web::get().to(list_users))
        .route("/admin/users/{id}", web::get().to(get_user_detail))
        .route(
            "/admin/users/{id}/transactions",
            web::get().to(list_user_transactions),
        )
        .route(
            "/admin/users/{id}/conversations",
            web::get().to(list_user_conversations),
        )
        .route(
            "/admin/users/{id}/safety-events",
            web::get().to(list_user_safety_events),
        )
        .route("/admin/conversations", web::get().to(list_all_conversations))
        .route(
            "/admin/conversations/{id}",
            web::get().to(get_conversation_detail),
        )
        .route("/admin/agent-rules", web::get().to(list_agent_rules))
        .route("/admin/agent-rules", web::post().to(create_agent_rule))
        .route(
            "/admin/agent-rules/{id}",
            web::patch().to(update_agent_rule),
        )
        .route(
            "/admin/agent-rules/{id}",
            web::delete().to(delete_agent_rule),
        )
        .route(
            "/admin/system-prompt-preview",
            web::get().to(system_prompt_preview),
        )
        .route("/admin/overview", web::get().to(overview));
}

#[derive(Deserialize)]
pub struct AuditQuery {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub action: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}
fn default_limit() -> i64 {
    200
}

async fn list_audit(
    state: web::Data<AppState>,
    auth: AuthUser,
    q: web::Query<AuditQuery>,
) -> Result<HttpResponse, ApiError> {
    let user = auth.0;
    let q = q.into_inner();
    let mut sql = String::from("SELECT * FROM audit_events WHERE 1=1");
    if user.role != "admin" {
        sql.push_str(" AND actor_user_id = ?");
    }
    if q.from.is_some() {
        sql.push_str(" AND created_at >= ?");
    }
    if q.to.is_some() {
        sql.push_str(" AND created_at <= ?");
    }
    if q.action.is_some() {
        sql.push_str(" AND action = ?");
    }
    sql.push_str(" ORDER BY created_at DESC LIMIT ?");
    let mut query = sqlx::query_as::<_, AuditEvent>(&sql);
    if user.role != "admin" {
        query = query.bind(&user.id);
    }
    if let Some(f) = q.from {
        query = query.bind(f);
    }
    if let Some(t) = q.to {
        query = query.bind(t);
    }
    if let Some(a) = q.action {
        query = query.bind(a);
    }
    query = query.bind(q.limit);
    let rows = query
        .fetch_all(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    let out: Vec<_> = rows
        .into_iter()
        .map(|ev| {
            let masked: serde_json::Value =
                serde_json::from_str(&ev.parameters_masked).unwrap_or(serde_json::json!({}));
            json!({
                "id": ev.id,
                "created_at": ev.created_at,
                "actor_user_id": ev.actor_user_id,
                "action": ev.action,
                "tool": ev.tool,
                "risk_tier": ev.risk_tier,
                "policy_reason": ev.policy_reason,
                "confirmation_status": ev.confirmation_status,
                "mfa_status": ev.mfa_status,
                "parameters_masked": masked,
                "result": ev.result,
                "error_code": ev.error_code,
            })
        })
        .collect();
    Ok(HttpResponse::Ok().json(out))
}

#[derive(Deserialize)]
pub struct ChainQuery {
    pub from_id: Option<String>,
    pub to_id: Option<String>,
}

async fn verify_chain(
    state: web::Data<AppState>,
    auth: AuthUser,
    q: web::Query<ChainQuery>,
) -> Result<HttpResponse, ApiError> {
    let _ = require_admin(&auth).await?;
    let q = q.into_inner();
    let r = audit::verify_chain(&state.pool, q.from_id.as_deref(), q.to_id.as_deref())
        .await
        .map_err(|_| errors::internal_error())?;
    Ok(HttpResponse::Ok().json(json!({
        "ok": r.ok,
        "events": r.events,
        "break_at": r.break_at,
        "reason": r.reason,
        "expected": r.expected,
        "actual": r.actual,
    })))
}

async fn list_safety_events(
    state: web::Data<AppState>,
    auth: AuthUser,
) -> Result<HttpResponse, ApiError> {
    let _ = require_admin(&auth).await?;
    let rows: Vec<SafetyEvent> = sqlx::query_as(
        "SELECT * FROM safety_events ORDER BY created_at DESC LIMIT 200",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    let out: Vec<_> = rows
        .into_iter()
        .map(|ev| {
            json!({
                "id": ev.id,
                "user_id": ev.user_id,
                "kind": ev.kind,
                "created_at": ev.created_at,
                "metadata": serde_json::from_str::<serde_json::Value>(&ev.metadata).unwrap_or(serde_json::json!({})),
            })
        })
        .collect();
    Ok(HttpResponse::Ok().json(out))
}

#[derive(Deserialize)]
pub struct UnlockIn {
    #[serde(default = "default_reason")]
    pub reason: String,
}
fn default_reason() -> String {
    "Manual admin unlock".to_string()
}

async fn admin_unlock(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    body: web::Json<UnlockIn>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let admin = require_admin(&auth).await?.0;
    let user_id = path.into_inner();
    let body = body.into_inner();
    let target: Option<User> = sqlx::query_as("SELECT * FROM users WHERE id = $1")
        .bind(&user_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    let mut target = target.ok_or_else(|| errors::not_found("User not found."))?;
    safety::unlock_user(&state.pool, &mut target, &admin.id, &body.reason)
        .await
        .map_err(|_| errors::internal_error())?;
    crate::audit::append_event(
        &state.pool,
        crate::audit::AppendEventParams {
            action: "admin.safety_unlock",
            actor_user_id: Some(&admin.id),
            session: None,
            request_id: Some(&rid.0),
            workflow_id: None,
            node_id: None,
            tool: None,
            parameters_masked: Some(json!({ "target_user_id": target.id, "reason": body.reason })),
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
        "user_id": target.id,
        "safety_lock_state": target.safety_lock_state,
    })))
}

async fn admin_reset(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let admin = require_admin(&auth).await?.0;
    let _ = req;
    crate::seed::reset_and_seed(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    crate::audit::append_event(
        &state.pool,
        crate::audit::AppendEventParams {
            action: "admin.reset",
            actor_user_id: Some(&admin.id),
            session: None,
            request_id: Some(&rid.0),
            workflow_id: None,
            node_id: None,
            tool: None,
            parameters_masked: None,
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
    Ok(HttpResponse::Ok().json(json!({ "ok": true })))
}

// ---------------------------------------------------------------------------
// Admin overview endpoints (PG-correct placeholders).
// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow, serde::Serialize)]
struct UserRow {
    id: String,
    username: String,
    display_name: String,
    preferred_language: String,
    role: String,
    safety_lock_state: String,
    safety_lock_reason: Option<String>,
    failed_mfa_count: i32,
    risk_tolerance: String,
    horizon: String,
    created_at: DateTime<Utc>,
    account_count: i64,
    transaction_count: i64,
    locked_event_count: i64,
}

async fn list_users(
    state: web::Data<AppState>,
    auth: AuthUser,
) -> Result<HttpResponse, ApiError> {
    let _ = require_admin(&auth).await?;
    let rows: Vec<UserRow> = sqlx::query_as(
        r#"
        SELECT
            u.id, u.username, u.display_name, u.preferred_language, u.role,
            u.safety_lock_state, u.safety_lock_reason, u.failed_mfa_count,
            u.risk_tolerance, u.horizon, u.created_at,
            COALESCE(ac.cnt, 0)  AS account_count,
            COALESCE(tx.cnt, 0)  AS transaction_count,
            COALESCE(sv.cnt, 0)  AS locked_event_count
        FROM users u
        LEFT JOIN (
            SELECT user_id, COUNT(*) AS cnt FROM accounts GROUP BY user_id
        ) ac ON ac.user_id = u.id
        LEFT JOIN (
            SELECT a.user_id, COUNT(*) AS cnt
            FROM transactions t JOIN accounts a ON a.id = t.account_id
            GROUP BY a.user_id
        ) tx ON tx.user_id = u.id
        LEFT JOIN (
            SELECT user_id, COUNT(*) AS cnt
            FROM safety_events
            WHERE kind IN ('denied_high_risk','rapid_high_value','injection_pattern','failed_mfa')
            GROUP BY user_id
        ) sv ON sv.user_id = u.id
        ORDER BY u.role DESC, u.created_at ASC
        "#,
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        log::error!("admin/users query failed: {e}");
        errors::internal_error()
    })?;
    Ok(HttpResponse::Ok().json(rows))
}

async fn overview(
    state: web::Data<AppState>,
    auth: AuthUser,
) -> Result<HttpResponse, ApiError> {
    let _ = require_admin(&auth).await?;
    let _rid = RequestId("admin.overview".to_string());

    let user_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&state.pool)
            .await
            .map_err(|_| errors::internal_error())?;
    let customer_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE role = $1",
    )
    .bind("customer")
    .fetch_one(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    let admin_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE role = $1",
    )
    .bind("admin")
    .fetch_one(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    let locked_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE safety_lock_state = $1",
    )
    .bind("locked")
    .fetch_one(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    let tx_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM transactions")
            .fetch_one(&state.pool)
            .await
            .map_err(|_| errors::internal_error())?;
    let safety_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM safety_events")
            .fetch_one(&state.pool)
            .await
            .map_err(|_| errors::internal_error())?;
    let audit_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM audit_events")
            .fetch_one(&state.pool)
            .await
            .map_err(|_| errors::internal_error())?;
    let chain = audit::verify_chain(&state.pool, None, None)
        .await
        .map_err(|_| errors::internal_error())?;
    let recent_safety: Vec<SafetyEvent> = sqlx::query_as(
        "SELECT * FROM safety_events ORDER BY created_at DESC LIMIT $1",
    )
    .bind(8_i64)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    let locked_users: Vec<User> = sqlx::query_as(
        "SELECT * FROM users WHERE safety_lock_state = $1 ORDER BY created_at DESC",
    )
    .bind("locked")
    .fetch_all(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;

    let recent_safety_json: Vec<_> = recent_safety
        .into_iter()
        .map(|ev| {
            json!({
                "id": ev.id,
                "user_id": ev.user_id,
                "kind": ev.kind,
                "created_at": ev.created_at,
                "metadata": serde_json::from_str::<serde_json::Value>(&ev.metadata)
                    .unwrap_or(serde_json::json!({})),
            })
        })
        .collect();

    let locked_users_json: Vec<_> = locked_users
        .into_iter()
        .map(|u| {
            json!({
                "id": u.id,
                "username": u.username,
                "display_name": u.display_name,
                "safety_lock_state": u.safety_lock_state,
                "safety_lock_reason": u.safety_lock_reason,
                "failed_mfa_count": u.failed_mfa_count,
                "role": u.role,
            })
        })
        .collect();

    Ok(HttpResponse::Ok().json(json!({
        "counts": {
            "users": user_count,
            "customers": customer_count,
            "admins": admin_count,
            "locked": locked_count,
            "transactions": tx_count,
            "safety_events": safety_count,
            "audit_events": audit_count,
        },
        "chain": {
            "ok": chain.ok,
            "events": chain.events,
            "break_at": chain.break_at,
            "reason": chain.reason,
        },
        "recent_safety": recent_safety_json,
        "locked_users": locked_users_json,
    })))
}
// ============================================================================
// Admin user-detail + monitoring endpoints
// ============================================================================

#[derive(serde::Serialize)]
struct UserDetail {
    #[serde(flatten)]
    user: User,
    accounts: Vec<Account>,
    transaction_count: i64,
    conversation_count: i64,
    safety_event_count: i64,
    balance_total_fen: i64,
}

async fn get_user_detail(
    state: web::Data<AppState>,
    auth: AuthUser,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let _admin = require_admin(&auth).await?;
    let user_id = path.into_inner();
    let user: User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
        .bind(&user_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?
        .ok_or_else(|| errors::not_found("User not found"))?;
    let accounts: Vec<Account> = sqlx::query_as(
        "SELECT * FROM accounts WHERE user_id = $1 ORDER BY created_at ASC",
    )
    .bind(&user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    let transaction_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM transactions t JOIN accounts a ON a.id = t.account_id WHERE a.user_id = $1",
    )
    .bind(&user_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    let conversation_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM conversations WHERE user_id = $1",
    )
    .bind(&user_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    let safety_event_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM safety_events WHERE user_id = $1",
    )
    .bind(&user_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    let balance_total_fen: i64 = accounts.iter().map(|a| a.available_balance_fen).sum();
    Ok(HttpResponse::Ok().json(UserDetail {
        user,
        accounts,
        transaction_count,
        conversation_count,
        safety_event_count,
        balance_total_fen,
    }))
}

#[derive(Deserialize)]
struct TxQuery {
    #[serde(default = "default_tx_limit")]
    limit: i64,
}
fn default_tx_limit() -> i64 {
    50
}

async fn list_user_transactions(
    state: web::Data<AppState>,
    auth: AuthUser,
    path: web::Path<String>,
    q: web::Query<TxQuery>,
) -> Result<HttpResponse, ApiError> {
    let _admin = require_admin(&auth).await?;
    let user_id = path.into_inner();
    let limit = q.limit.clamp(1, 500);
    let rows: Vec<Transaction> = sqlx::query_as(
        "SELECT t.* FROM transactions t \
         JOIN accounts a ON a.id = t.account_id \
         WHERE a.user_id = $1 \
         ORDER BY t.occurred_at DESC LIMIT $2",
    )
    .bind(&user_id)
    .bind(limit)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    Ok(HttpResponse::Ok().json(json!({ "transactions": rows, "limit": limit })))
}

async fn list_user_conversations(
    state: web::Data<AppState>,
    auth: AuthUser,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let _admin = require_admin(&auth).await?;
    let user_id = path.into_inner();
    let convs: Vec<Conversation> = sqlx::query_as(
        "SELECT * FROM conversations WHERE user_id = $1 ORDER BY last_message_at DESC LIMIT 50",
    )
    .bind(&user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    Ok(HttpResponse::Ok().json(json!({ "conversations": convs })))
}

async fn list_user_safety_events(
    state: web::Data<AppState>,
    auth: AuthUser,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let _admin = require_admin(&auth).await?;
    let user_id = path.into_inner();
    let events: Vec<SafetyEvent> = sqlx::query_as(
        "SELECT * FROM safety_events WHERE user_id = $1 ORDER BY created_at DESC LIMIT 200",
    )
    .bind(&user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    Ok(HttpResponse::Ok().json(json!({ "events": events })))
}

async fn list_all_conversations(
    state: web::Data<AppState>,
    auth: AuthUser,
) -> Result<HttpResponse, ApiError> {
    let _admin = require_admin(&auth).await?;
    let convs: Vec<Conversation> = sqlx::query_as(
        "SELECT * FROM conversations ORDER BY last_message_at DESC LIMIT 100",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    Ok(HttpResponse::Ok().json(json!({ "conversations": convs })))
}

async fn get_conversation_detail(
    state: web::Data<AppState>,
    auth: AuthUser,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let _admin = require_admin(&auth).await?;
    let conv_id = path.into_inner();
    let conv: Option<Conversation> = sqlx::query_as("SELECT * FROM conversations WHERE id = $1")
        .bind(&conv_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    let conv = conv.ok_or_else(|| errors::not_found("Conversation not found"))?;
    let messages: Vec<ConversationMessage> = sqlx::query_as(
        "SELECT * FROM conversation_messages WHERE conversation_id = $1 ORDER BY created_at ASC",
    )
    .bind(&conv_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    Ok(HttpResponse::Ok().json(json!({
        "conversation": conv,
        "messages": messages,
    })))
}

// ============================================================================
// Agent rules engine — admin CRUD + system-prompt preview
// ============================================================================

async fn list_agent_rules(
    state: web::Data<AppState>,
    auth: AuthUser,
) -> Result<HttpResponse, ApiError> {
    let _admin = require_admin(&auth).await?;
    let rules = agent_rules::list_rules(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    Ok(HttpResponse::Ok().json(json!({ "rules": rules })))
}

async fn create_agent_rule(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    body: web::Json<NewAgentRule>,
) -> Result<HttpResponse, ApiError> {
    let admin = require_admin(&auth).await?.0;
    require_csrf(&req, &auth).await?;
    let rule = agent_rules::create_rule(&state.pool, &admin, body.into_inner())
        .await?;
    audit::append_event(
        &state.pool,
        audit::AppendEventParams {
            action: "admin.agent_rule.create",
            actor_user_id: Some(&admin.id),
            session: None,
            request_id: Some(&rid.0),
            workflow_id: None,
            node_id: None,
            tool: None,
            parameters_masked: Some(json!({ "id": rule.id, "kind": rule.kind })),
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
    Ok(HttpResponse::Ok().json(rule))
}

async fn update_agent_rule(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    body: web::Json<UpdateAgentRule>,
) -> Result<HttpResponse, ApiError> {
    let admin = require_admin(&auth).await?.0;
    require_csrf(&req, &auth).await?;
    let id = path.into_inner();
    let rule = agent_rules::update_rule(&state.pool, &admin, &id, body.into_inner())
        .await?;
    audit::append_event(
        &state.pool,
        audit::AppendEventParams {
            action: "admin.agent_rule.update",
            actor_user_id: Some(&admin.id),
            session: None,
            request_id: Some(&rid.0),
            workflow_id: None,
            node_id: None,
            tool: None,
            parameters_masked: Some(json!({ "id": id })),
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
    Ok(HttpResponse::Ok().json(rule))
}

async fn delete_agent_rule(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let admin = require_admin(&auth).await?.0;
    require_csrf(&req, &auth).await?;
    let id = path.into_inner();
    agent_rules::delete_rule(&state.pool, &admin, &id).await?;
    audit::append_event(
        &state.pool,
        audit::AppendEventParams {
            action: "admin.agent_rule.delete",
            actor_user_id: Some(&admin.id),
            session: None,
            request_id: Some(&rid.0),
            workflow_id: None,
            node_id: None,
            tool: None,
            parameters_masked: Some(json!({ "id": id })),
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
    Ok(HttpResponse::Ok().json(json!({ "ok": true, "id": id })))
}

#[derive(Deserialize)]
struct PreviewQuery {
    user_id: Option<String>,
}

async fn system_prompt_preview(
    state: web::Data<AppState>,
    auth: AuthUser,
    q: web::Query<PreviewQuery>,
) -> Result<HttpResponse, ApiError> {
    let _admin = require_admin(&auth).await?;
    let user_id = q.user_id.clone().unwrap_or_else(|| auth.0.id.clone());
    let prefix = agent_rules::system_prompt_prefix(&state.pool, &user_id).await;
    let snapshot = agent_rules::snapshot(&state.pool).await;
    let forbidden = agent_rules::forbidden_tools(&state.pool, &user_id).await;
    Ok(HttpResponse::Ok().json(json!({
        "user_id": user_id,
        "prefix": prefix,
        "snapshot": agent_rules::snapshot_to_json(&snapshot),
        "forbidden_tools": forbidden,
    })))
}
