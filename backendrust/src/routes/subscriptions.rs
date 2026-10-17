//! Subscription + reminder routes (Scenario 3).

use actix_web::{web, HttpResponse};
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;

use crate::audit::{self, AppendEventParams};
use crate::ctx::{require_csrf, AuthUser, RequestId};
use crate::errors::{self, ApiError};
use crate::ids;
use crate::models::{Reminder, Subscription};
use crate::policy;

use crate::state::AppState;
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/subscriptions", web::get().to(list_subscriptions))
        .route(
            "/subscriptions/{id}/remind",
            web::post().to(remind_subscription),
        )
        .route(
            "/subscriptions/{id}/cancel",
            web::post().to(cancel_subscription),
        )
        .route("/reminders", web::get().to(list_reminders))
        .route("/reminders/{id}", web::delete().to(delete_reminder));
}

async fn list_subscriptions(
    state: web::Data<AppState>,
    auth: AuthUser,
) -> Result<HttpResponse, ApiError> {
    let user = auth.0;
    let rows: Vec<Subscription> = sqlx::query_as(
        "SELECT id, user_id, merchant, estimated_amount_fen, interval_days, next_expected_at, last_charge_at, evidence::text AS evidence, cancellation_status, cancelled_at, created_at FROM subscriptions WHERE user_id = $1 ORDER BY merchant ASC",
    )
    .bind(&user.id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    let out: Vec<_> = rows
        .into_iter()
        .map(|s| {
            json!({
                "id": s.id,
                "merchant": s.merchant,
                "estimated_amount_fen": s.estimated_amount_fen,
                "interval_days": s.interval_days,
                "next_expected_at": s.next_expected_at,
                "last_charge_at": s.last_charge_at,
                "cancellation_status": s.cancellation_status,
                "evidence": s.evidence,
            })
        })
        .collect();
    Ok(HttpResponse::Ok().json(out))
}

#[derive(Deserialize)]
pub struct RemindQuery {
    #[serde(default = "default_days")]
    pub days_before: i64,
}
fn default_days() -> i64 {
    3
}

async fn remind_subscription(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    q: web::Query<RemindQuery>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let user = auth.0;
    let id = path.into_inner();
    let sub: Option<Subscription> = sqlx::query_as("SELECT id, user_id, merchant, estimated_amount_fen, interval_days, next_expected_at, last_charge_at, evidence::text AS evidence, cancellation_status, cancelled_at, created_at FROM subscriptions WHERE id = $1")
        .bind(&id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    let sub = sub.ok_or_else(|| errors::not_found("Subscription not found."))?;
    if sub.user_id != user.id {
        return Err(errors::not_found("Subscription not found."));
    }
    let due = sub.next_expected_at;
    let remind_at = due - Duration::days(q.days_before);
    let rem_id = ids::new_id("rem");
    let now = Utc::now();
    sqlx::query(
        "INSERT INTO reminders (id, user_id, kind, subject, due_at, status, linked_subscription_id, created_at) VALUES ($1, $2, 'renewal', $3, $4, 'scheduled', $5, $6)",
    )
    .bind(&rem_id)
    .bind(&user.id)
    .bind(format!("Subscription '{}' renews soon", sub.merchant))
    .bind(remind_at)
    .bind(&sub.id)
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    audit::append_event(
        &state.pool,
        AppendEventParams {
            action: "subscription.remind",
            actor_user_id: Some(&user.id),
            session: None,
            request_id: Some(&rid.0),
            workflow_id: None,
            node_id: None,
            tool: Some("subscriptions.remind"),
            parameters_masked: Some(json!({ "subscription_id": sub.id, "days_before": q.days_before })),
            risk_tier: Some("Green"),
            policy_reason: None,
            confirmation_status: None,
            mfa_status: None,
            result: "ok",
            error_code: None,
        },
    )
    .await
    .ok();
    let rem: Reminder = sqlx::query_as("SELECT * FROM reminders WHERE id = $1")
        .bind(&rem_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    Ok(HttpResponse::Ok().json(json!({
        "id": rem.id,
        "kind": rem.kind,
        "subject": rem.subject,
        "due_at": rem.due_at,
        "status": rem.status,
        "linked_subscription_id": rem.linked_subscription_id,
    })))
}

async fn cancel_subscription(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let user = auth.0;
    let id = path.into_inner();
    let sub: Option<Subscription> = sqlx::query_as("SELECT id, user_id, merchant, estimated_amount_fen, interval_days, next_expected_at, last_charge_at, evidence::text AS evidence, cancellation_status, cancelled_at, created_at FROM subscriptions WHERE id = $1")
        .bind(&id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    let sub = sub.ok_or_else(|| errors::not_found("Subscription not found."))?;
    if sub.user_id != user.id {
        return Err(errors::not_found("Subscription not found."));
    }
    if matches!(
        sub.cancellation_status.as_str(),
        "cancellation_requested" | "cancelled"
    ) {
        return Ok(HttpResponse::Ok().json(json!({
            "subscription_id": sub.id,
            "status": sub.cancellation_status,
        })));
    }
    let now = Utc::now();
    sqlx::query("UPDATE subscriptions SET cancellation_status = 'cancellation_requested', cancelled_at = $1 WHERE id = $2")
        .bind(now)
        .bind(&sub.id)
        .execute(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    sqlx::query("UPDATE subscriptions SET cancellation_status = 'cancelled' WHERE id = $1")
        .bind(&sub.id)
        .execute(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    audit::append_event(
        &state.pool,
        AppendEventParams {
            action: "subscription.cancel",
            actor_user_id: Some(&user.id),
            session: None,
            request_id: Some(&rid.0),
            workflow_id: None,
            node_id: None,
            tool: Some("subscriptions.cancel"),
            parameters_masked: Some(json!({ "subscription_id": sub.id, "merchant": sub.merchant })),
            risk_tier: Some(policy::decide_subscription_cancel().tier.as_str()),
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
        "subscription_id": sub.id,
        "status": "cancelled",
    })))
}

async fn list_reminders(
    state: web::Data<AppState>,
    auth: AuthUser,
) -> Result<HttpResponse, ApiError> {
    let user = auth.0;
    let rows: Vec<Reminder> = sqlx::query_as(
        "SELECT * FROM reminders WHERE user_id = $1 ORDER BY due_at ASC",
    )
    .bind(&user.id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    let out: Vec<_> = rows
        .into_iter()
        .map(|r| {
            json!({
                "id": r.id,
                "kind": r.kind,
                "subject": r.subject,
                "due_at": r.due_at,
                "status": r.status,
                "linked_subscription_id": r.linked_subscription_id,
            })
        })
        .collect();
    Ok(HttpResponse::Ok().json(out))
}

async fn delete_reminder(
    state: web::Data<AppState>,
    auth: AuthUser,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let user = auth.0;
    let id = path.into_inner();
    let row: Option<Reminder> = sqlx::query_as("SELECT * FROM reminders WHERE id = $1")
        .bind(&id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    let row = row.ok_or_else(|| errors::not_found("Reminder not found."))?;
    if row.user_id != user.id {
        return Err(errors::not_found("Reminder not found."));
    }
    sqlx::query("UPDATE reminders SET status = 'cancelled' WHERE id = $1")
        .bind(&id)
        .execute(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    Ok(HttpResponse::Ok().json(json!({
        "reminder_id": id,
        "status": "cancelled",
    })))
}
