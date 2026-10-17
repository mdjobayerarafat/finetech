//! Pending-action routes: confirm / verify / execute.

use actix_web::{web, HttpResponse};
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;

use crate::audit::{self, AppendEventParams};
use crate::canonical::parameter_hash;
use crate::config;
use crate::ctx::{require_csrf, AuthUser, RequestId};
use crate::errors::{self, ApiError};
use crate::models::{PendingAction, Workflow, WorkflowNode};
use crate::safety;
use crate::tools::{self, ToolContext, ToolError};

use crate::state::AppState;
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/actions/{id}", web::get().to(get_action))
        .route("/actions/{id}/verify", web::post().to(verify_action))
        .route("/actions/{id}/confirm", web::post().to(confirm_action))
        .route("/actions/{id}/execute", web::post().to(execute_action));
}

#[derive(Deserialize)]
pub struct VerifyIn {
    pub code: String,
}

async fn fetch_action(
    pool: &PgPool,
    user_id: &str,
    id: &str,
) -> Result<PendingAction, ApiError> {
    let row: Option<PendingAction> = sqlx::query_as("SELECT * FROM pending_actions WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|_| errors::internal_error())?;
    let row = row.ok_or_else(|| errors::not_found("Action not found."))?;
    if row.user_id != user_id {
        return Err(errors::not_found("Action not found."));
    }
    Ok(row)
}

fn is_expired(pa: &PendingAction) -> bool {
    pa.expires_at <= Utc::now()
}

async fn get_action(
    state: web::Data<AppState>,
    auth: AuthUser,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let user = auth.0;
    let id = path.into_inner();
    let mut pa = fetch_action(&state.pool, &user.id, &id).await?;
    if is_expired(&pa) && pa.confirmation_status == "none" {
        sqlx::query("UPDATE pending_actions SET confirmation_status = 'expired' WHERE id = $1")
            .bind(&pa.id)
            .execute(&state.pool)
            .await
            .ok();
        pa.confirmation_status = "expired".to_string();
    }
    Ok(HttpResponse::Ok().json(json!({
        "id": pa.id,
        "tool": pa.tool,
        "parameters": serde_json::from_str::<serde_json::Value>(&pa.parameters).unwrap_or(serde_json::json!({})),
        "parameter_hash": pa.parameter_hash,
        "risk_tier": pa.risk_tier,
        "expires_at": pa.expires_at,
        "confirmation_status": pa.confirmation_status,
        "mfa_status": pa.mfa_status,
    })))
}

async fn verify_action(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    body: web::Json<VerifyIn>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let user = auth.0;
    let id = path.into_inner();
    let body = body.into_inner();
    let mut pa = fetch_action(&state.pool, &user.id, &id).await?;
    if pa.risk_tier != "Red" {
        return Err(errors::ApiError::new(
            "schema_invalid",
            "This action does not require MFA verification.",
            400,
        ));
    }
    if is_expired(&pa) {
        sqlx::query("UPDATE pending_actions SET confirmation_status = 'expired', mfa_status = 'expired' WHERE id = $1")
            .bind(&pa.id)
            .execute(&state.pool)
            .await
            .ok();
        pa.confirmation_status = "expired".into();
        pa.mfa_status = "expired".into();
        audit::append_event(
            &state.pool,
            AppendEventParams {
                action: "action.verify",
                actor_user_id: Some(&user.id),
                session: None,
                request_id: Some(&rid.0),
                workflow_id: None,
                node_id: None,
                tool: Some(&pa.tool),
                parameters_masked: Some(
                    serde_json::from_str::<serde_json::Value>(&pa.parameters)
                        .unwrap_or(serde_json::json!({})),
                ),
                risk_tier: Some(&pa.risk_tier),
                policy_reason: None,
                confirmation_status: None,
                mfa_status: Some("expired"),
                result: "denied",
                error_code: Some("expired"),
            },
        )
        .await
        .ok();
        return Err(errors::expired());
    }
    let settings = config::get_settings();
    if body.code != settings.mfa_demo_code {
        let mut user_mut = user.clone();
        let res = safety::record_failed_mfa(&state.pool, &mut user_mut)
            .await
            .map_err(|_| errors::internal_error())?;
        audit::append_event(
            &state.pool,
            AppendEventParams {
                action: "action.verify",
                actor_user_id: Some(&user.id),
                session: None,
                request_id: Some(&rid.0),
                workflow_id: None,
                node_id: None,
                tool: Some(&pa.tool),
                parameters_masked: Some(
                    serde_json::from_str::<serde_json::Value>(&pa.parameters)
                        .unwrap_or(serde_json::json!({})),
                ),
                risk_tier: Some(&pa.risk_tier),
                policy_reason: None,
                confirmation_status: None,
                mfa_status: Some("failed"),
                result: "denied",
                error_code: Some("mfa_failed"),
            },
        )
        .await
        .ok();
        return Err(errors::ApiError::new("mfa_failed", "The MFA code is incorrect.", 403)
            .with_details(json!({
                "locked": res.locked,
                "failed_mfa_count": res.failed_mfa_count,
            })));
    }
    sqlx::query("UPDATE pending_actions SET mfa_status = 'verified' WHERE id = $1")
        .bind(&pa.id)
        .execute(&state.pool)
        .await
        .ok();
    pa.mfa_status = "verified".into();
    let mut user_mut = user.clone();
    safety::record_successful_mfa(&state.pool, &mut user_mut)
        .await
        .ok();
    audit::append_event(
        &state.pool,
        AppendEventParams {
            action: "action.verify",
            actor_user_id: Some(&user.id),
            session: None,
            request_id: Some(&rid.0),
            workflow_id: None,
            node_id: None,
            tool: Some(&pa.tool),
            parameters_masked: Some(
                serde_json::from_str::<serde_json::Value>(&pa.parameters)
                    .unwrap_or(serde_json::json!({})),
            ),
            risk_tier: Some(&pa.risk_tier),
            policy_reason: None,
            confirmation_status: None,
            mfa_status: Some("verified"),
            result: "ok",
            error_code: None,
        },
    )
    .await
    .ok();
    Ok(HttpResponse::Ok().json(json!({
        "pending_action_id": pa.id,
        "confirmation_status": pa.confirmation_status,
        "mfa_status": pa.mfa_status,
        "workflow_id": null,
        "workflow_node_id": pa.workflow_node_id,
        "executed": false,
    })))
}

#[derive(Deserialize, Default)]
pub struct ConfirmIn {
    pub parameter_hash: Option<String>,
}

async fn confirm_action(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    body: Option<web::Json<ConfirmIn>>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let user = auth.0;
    let id = path.into_inner();
    let body = body.map(|b| b.into_inner()).unwrap_or_default();
    let mut pa = fetch_action(&state.pool, &user.id, &id).await?;
    safety::assert_unlocked(&user)?;
    if is_expired(&pa) {
        sqlx::query("UPDATE pending_actions SET confirmation_status = 'expired' WHERE id = $1")
            .bind(&pa.id)
            .execute(&state.pool)
            .await
            .ok();
        if pa.risk_tier == "Red" {
            sqlx::query("UPDATE pending_actions SET mfa_status = 'expired' WHERE id = $1")
                .bind(&pa.id)
                .execute(&state.pool)
                .await
                .ok();
        }
        audit::append_event(
            &state.pool,
            AppendEventParams {
                action: "action.confirm",
                actor_user_id: Some(&user.id),
                session: None,
                request_id: Some(&rid.0),
                workflow_id: None,
                node_id: None,
                tool: Some(&pa.tool),
                parameters_masked: Some(
                    serde_json::from_str::<serde_json::Value>(&pa.parameters)
                        .unwrap_or(serde_json::json!({})),
                ),
                risk_tier: Some(&pa.risk_tier),
                policy_reason: None,
                confirmation_status: None,
                mfa_status: None,
                result: "denied",
                error_code: Some("expired"),
            },
        )
        .await
        .ok();
        return Err(errors::expired());
    }
    if pa.risk_tier == "Red" && pa.mfa_status != "verified" {
        return Err(errors::ApiError::new(
            "mfa_required",
            "Mock MFA verification is required first.",
            403,
        ));
    }
    if let Some(hash_override) = body.parameter_hash {
        if hash_override != pa.parameter_hash {
            sqlx::query("UPDATE pending_actions SET confirmation_status = 'invalidated' WHERE id = $1")
                .bind(&pa.id)
                .execute(&state.pool)
                .await
                .ok();
            audit::append_event(
                &state.pool,
                AppendEventParams {
                    action: "action.confirm",
                    actor_user_id: Some(&user.id),
                    session: None,
                    request_id: Some(&rid.0),
                    workflow_id: None,
                    node_id: None,
                    tool: Some(&pa.tool),
                    parameters_masked: Some(
                        serde_json::from_str::<serde_json::Value>(&pa.parameters)
                            .unwrap_or(serde_json::json!({})),
                    ),
                    risk_tier: Some(&pa.risk_tier),
                    policy_reason: None,
                    confirmation_status: Some("invalidated"),
                    mfa_status: None,
                    result: "denied",
                    error_code: Some("parameter_changed"),
                },
            )
            .await
            .ok();
            return Err(errors::parameter_changed());
        }
    }
    sqlx::query("UPDATE pending_actions SET confirmation_status = 'confirmed' WHERE id = $1")
        .bind(&pa.id)
        .execute(&state.pool)
        .await
        .ok();
    pa.confirmation_status = "confirmed".into();

    let mut args: serde_json::Value =
        serde_json::from_str(&pa.parameters).unwrap_or(serde_json::json!({}));
    let mut node: Option<WorkflowNode> = None;
    if let Some(wnid) = &pa.workflow_node_id {
        let n: Option<WorkflowNode> = sqlx::query_as("SELECT * FROM workflow_nodes WHERE id = $1")
            .bind(wnid)
            .fetch_optional(&state.pool)
            .await
            .map_err(|_| errors::internal_error())?;
        node = n;
        if let Some(ref n) = node {
            if let Some(idem) = Some(&n.idempotency_key) {
                if !idem.is_empty() {
                    if let serde_json::Value::Object(ref mut m) = args {
                        m.entry("idempotency_key".to_string())
                            .or_insert(serde_json::Value::String(idem.clone()));
                    }
                }
            }
        }
    }

    let tool_fn = tools::registry().get(pa.tool.as_str()).copied();
    let mut executed = false;
    let mut result_payload: Option<serde_json::Value> = None;
    if let (Some(tool_fn), Some(ref n)) = (tool_fn, node.as_ref()) {
        let ctx = ToolContext {
            pool: state.pool.clone(),
            user: user.clone(),
            request_id: rid.0.clone(),
            session_id: None,
        };
        match tool_fn(&ctx, &args).await {
            Ok(v) => {
                let result_str = serde_json::to_string(&v).unwrap_or_else(|_| "{}".to_string());
                sqlx::query("UPDATE workflow_nodes SET status = 'completed', result = $1 WHERE id = $2")
                    .bind(&result_str)
                    .bind(&n.id)
                    .execute(&state.pool)
                    .await
                    .ok();
                executed = true;
                result_payload = Some(v);
                // Advance workflow status.
                let wf: Option<Workflow> = sqlx::query_as("SELECT * FROM workflows WHERE id = $1")
                    .bind(&n.workflow_id)
                    .fetch_optional(&state.pool)
                    .await
                    .map_err(|_| errors::internal_error())?;
                if let Some(wf) = wf {
                    let remaining: i64 = sqlx::query_scalar(
                        "SELECT COUNT(*) FROM workflow_nodes WHERE workflow_id = $1 AND status IN ('pending','awaiting_confirmation','awaiting_mfa','paused')",
                    )
                    .bind(&wf.id)
                    .fetch_one(&state.pool)
                    .await
                    .map_err(|_| errors::internal_error())?;
                    let new_status = if remaining == 0 { "completed" } else { "awaiting_confirmation" };
                    sqlx::query("UPDATE workflows SET status = $1 WHERE id = $2")
                        .bind(new_status)
                        .bind(&wf.id)
                        .execute(&state.pool)
                        .await
                        .ok();
                }
            }
            Err(e) => {
                let err_str = serde_json::to_string(&json!({
                    "code": e.code(),
                    "message": e.to_string(),
                }))
                .unwrap_or("{}".to_string());
                sqlx::query("UPDATE workflow_nodes SET status = 'failed', error = $1 WHERE id = $2")
                    .bind(&err_str)
                    .bind(&n.id)
                    .execute(&state.pool)
                    .await
                    .ok();
                audit::append_event(
                    &state.pool,
                    AppendEventParams {
                        action: "action.execute",
                        actor_user_id: Some(&user.id),
                        session: None,
                        request_id: Some(&rid.0),
                        workflow_id: Some(&n.workflow_id),
                        node_id: Some(&n.id),
                        tool: Some(&pa.tool),
                        parameters_masked: Some(args.clone()),
                        risk_tier: Some(&pa.risk_tier),
                        policy_reason: None,
                        confirmation_status: Some("confirmed"),
                        mfa_status: Some(&pa.mfa_status),
                        result: "error",
                        error_code: Some(e.code()),
                    },
                )
                .await
                .ok();
                return Err(tool_err_to_api(e));
            }
        }
    }
    audit::append_event(
        &state.pool,
        AppendEventParams {
            action: "action.confirm",
            actor_user_id: Some(&user.id),
            session: None,
            request_id: Some(&rid.0),
            workflow_id: node.as_ref().map(|n| n.workflow_id.as_str()),
            node_id: node.as_ref().map(|n| n.id.as_str()),
            tool: Some(&pa.tool),
            parameters_masked: Some(args),
            risk_tier: Some(&pa.risk_tier),
            policy_reason: None,
            confirmation_status: Some("confirmed"),
            mfa_status: Some(&pa.mfa_status),
            result: "ok",
            error_code: None,
        },
    )
    .await
    .ok();
    Ok(HttpResponse::Ok().json(json!({
        "pending_action_id": pa.id,
        "confirmation_status": pa.confirmation_status,
        "mfa_status": pa.mfa_status,
        "workflow_id": node.as_ref().map(|n| &n.workflow_id),
        "workflow_node_id": node.as_ref().map(|n| &n.id),
        "executed": executed,
        "result": result_payload,
    })))
}

async fn execute_action(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let user = auth.0;
    let id = path.into_inner();
    let pa = fetch_action(&state.pool, &user.id, &id).await?;
    safety::assert_unlocked(&user)?;
    if pa.confirmation_status != "confirmed" {
        return Err(errors::ApiError::new(
            "schema_invalid",
            "Action must be confirmed before execution.",
            409,
        ));
    }
    if pa.risk_tier == "Red" && pa.mfa_status != "verified" {
        return Err(errors::ApiError::new(
            "mfa_required",
            "Mock MFA verification is required.",
            403,
        ));
    }
    let node_id = pa
        .workflow_node_id
        .as_ref()
        .ok_or_else(|| errors::ApiError::new("schema_invalid", "This action is not attached to a workflow node.", 400))?;
    let n: Option<WorkflowNode> = sqlx::query_as("SELECT * FROM workflow_nodes WHERE id = $1")
        .bind(node_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    let n = n.ok_or_else(|| errors::not_found("Action not found."))?;
    let args: serde_json::Value =
        serde_json::from_str(&pa.parameters).unwrap_or(serde_json::json!({}));
    let tool_fn = tools::registry().get(pa.tool.as_str()).copied().ok_or_else(|| {
        errors::ApiError::new("schema_invalid", &format!("Tool {} not found.", pa.tool), 400)
    })?;
    let ctx = ToolContext {
        pool: state.pool.clone(),
        user: user.clone(),
        request_id: rid.0.clone(),
        session_id: None,
    };
    let result = tool_fn(&ctx, &args).await;
    match result {
        Ok(v) => {
            let result_str = serde_json::to_string(&v).unwrap_or_else(|_| "{}".to_string());
            sqlx::query("UPDATE workflow_nodes SET status = 'completed', result = $1 WHERE id = $2")
                .bind(&result_str)
                .bind(&n.id)
                .execute(&state.pool)
                .await
                .map_err(|_| errors::internal_error())?;
            Ok(HttpResponse::Ok().json(json!({
                "pending_action_id": pa.id,
                "confirmation_status": pa.confirmation_status,
                "mfa_status": pa.mfa_status,
                "workflow_id": n.workflow_id,
                "workflow_node_id": n.id,
                "executed": true,
                "result": v,
            })))
        }
        Err(e) => {
            let err_str = serde_json::to_string(&json!({
                "code": e.code(),
                "message": e.to_string(),
            }))
            .unwrap_or("{}".to_string());
            sqlx::query("UPDATE workflow_nodes SET status = 'failed', error = $1 WHERE id = $2")
                .bind(&err_str)
                .bind(&n.id)
                .execute(&state.pool)
                .await
                .ok();
            Err(tool_err_to_api(e))
        }
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
    errors::ApiError::new(code, &e.to_string(), status)
}

// silence unused
#[allow(dead_code)]
fn _silence(p: &PendingAction, h: &str) -> String {
    let _ = parameter_hash(&serde_json::json!({}));
    let _ = (p, h);
    String::new()
}
