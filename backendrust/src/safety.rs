//! Safety lock: track failed MFAs, denied high-risk attempts, and lock the user.

use chrono::{Duration, Utc};
use serde::Serialize;
use serde_json::Value;
use sqlx::PgPool;

use crate::audit::{self, AppendEventParams};
use crate::config;
use crate::ids;
use crate::models::{SafetyEvent, User};

#[derive(Debug, Clone, Serialize)]
pub struct FailedMfaResult {
    pub failed_mfa_count: i32,
    pub locked: bool,
}

pub fn is_locked(user: &User) -> bool {
    user.safety_lock_state == "locked"
}

pub fn assert_unlocked(user: &User) -> Result<(), crate::errors::ApiError> {
    if is_locked(user) {
        return Err(crate::errors::safety_locked(
            user.safety_lock_reason.as_deref(),
        ));
    }
    Ok(())
}

pub async fn record_failed_mfa(
    pool: &PgPool,
    user: &mut User,
) -> Result<FailedMfaResult, sqlx::Error> {
    let settings = config::get_settings();
    user.failed_mfa_count += 1;
    let will_lock = user.failed_mfa_count >= settings.max_failed_mfa && !is_locked(user);
    sqlx::query("UPDATE users SET failed_mfa_count = $1 WHERE id = $2")
        .bind(user.failed_mfa_count)
        .bind(&user.id)
        .execute(pool)
        .await?;
    if will_lock {
        lock_user(
            pool,
            user,
            "failed_mfa",
            format!("{} failed MFA attempts", user.failed_mfa_count),
        )
        .await?;
    }
    Ok(FailedMfaResult {
        failed_mfa_count: user.failed_mfa_count,
        locked: is_locked(user),
    })
}

pub async fn record_successful_mfa(pool: &PgPool, user: &mut User) -> Result<(), sqlx::Error> {
    user.failed_mfa_count = 0;
    sqlx::query("UPDATE users SET failed_mfa_count = 0 WHERE id = $1")
        .bind(&user.id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn record_injection(
    pool: &PgPool,
    user: &mut User,
    phrase: &str,
    where_: &str,
) -> Result<(), sqlx::Error> {
    emit(
        pool,
        user,
        "injection_pattern",
        serde_json::json!({ "phrase": phrase, "where": where_ }),
    )
    .await?;
    if !is_locked(user) {
        lock_user(pool, user, "injection_pattern", "Injection pattern detected".to_string()).await?;
    }
    Ok(())
}

pub async fn record_denied_high_risk(
    pool: &PgPool,
    user: &mut User,
    reason: &str,
) -> Result<(), sqlx::Error> {
    emit(
        pool,
        user,
        "denied_high_risk",
        serde_json::json!({ "reason": reason }),
    )
    .await?;
    let window_start = Utc::now() - Duration::minutes(5);
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM safety_events WHERE user_id = $1 AND kind = 'denied_high_risk' AND created_at >= $2",
    )
    .bind(&user.id)
    .bind(window_start)
    .fetch_one(pool)
    .await?;
    if count >= 2 && !is_locked(user) {
        lock_user(
            pool,
            user,
            "denied_high_risk",
            "Repeated denied high-risk actions".to_string(),
        )
        .await?;
    }
    Ok(())
}

pub async fn record_rapid_high_value(
    pool: &PgPool,
    user: &mut User,
    amount_fen: i64,
) -> Result<(), sqlx::Error> {
    let settings = config::get_settings();
    if amount_fen < settings.daily_limit_fen / 2 {
        return Ok(());
    }
    emit(
        pool,
        user,
        "rapid_high_value",
        serde_json::json!({ "amount_fen": amount_fen }),
    )
    .await?;
    let window_start = Utc::now() - Duration::minutes(5);
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM safety_events WHERE user_id = $1 AND kind = 'rapid_high_value' AND created_at >= $2",
    )
    .bind(&user.id)
    .bind(window_start)
    .fetch_one(pool)
    .await?;
    if count >= 3 && !is_locked(user) {
        lock_user(
            pool,
            user,
            "rapid_high_value",
            "Multiple high-value attempts in a short window".to_string(),
        )
        .await?;
    }
    Ok(())
}

pub async fn lock_user(
    pool: &PgPool,
    user: &mut User,
    kind: &str,
    reason: String,
) -> Result<(), sqlx::Error> {
    user.safety_lock_state = "locked".to_string();
    user.safety_lock_reason = Some(reason.clone());
    sqlx::query("UPDATE users SET safety_lock_state = 'locked', safety_lock_reason = $1 WHERE id = $2")
        .bind(&reason)
        .bind(&user.id)
        .execute(pool)
        .await?;
    emit(
        pool,
        user,
        kind,
        serde_json::json!({ "reason": reason }),
    )
    .await?;
    Ok(())
}

pub async fn unlock_user(
    pool: &PgPool,
    user: &mut User,
    admin_user_id: &str,
    reason: &str,
) -> Result<(), sqlx::Error> {
    user.safety_lock_state = "unlocked".to_string();
    user.safety_lock_reason = None;
    user.failed_mfa_count = 0;
    sqlx::query(
        "UPDATE users SET safety_lock_state = 'unlocked', safety_lock_reason = NULL, failed_mfa_count = 0 WHERE id = $1",
    )
    .bind(&user.id)
    .execute(pool)
    .await?;
    emit(
        pool,
        user,
        "admin_unlock",
        serde_json::json!({ "admin_user_id": admin_user_id, "reason": reason }),
    )
    .await?;
    Ok(())
}

async fn emit(
    pool: &PgPool,
    user: &User,
    kind: &str,
    metadata: Value,
) -> Result<(), sqlx::Error> {
    let now = Utc::now();
    let id = ids::new_id("safety");
    let metadata_str = serde_json::to_string(&metadata).unwrap_or("{}".to_string());
    sqlx::query(
        "INSERT INTO safety_events (id, user_id, kind, created_at, metadata) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(&id)
    .bind(&user.id)
    .bind(kind)
    .bind(now)
    .bind(&metadata_str)
    .execute(pool)
    .await?;
    let _ = audit::append_event(
        pool,
        AppendEventParams {
            action: "safety.event",
            actor_user_id: Some(&user.id),
            session: None,
            request_id: None,
            workflow_id: None,
            node_id: None,
            tool: None,
            parameters_masked: Some(serde_json::json!({ "kind": kind })),
            risk_tier: None,
            policy_reason: None,
            confirmation_status: None,
            mfa_status: None,
            result: "ok",
            error_code: None,
        },
    )
    .await?;
    let _ = SafetyEvent {
        id,
        user_id: user.id.clone(),
        kind: kind.to_string(),
        created_at: now,
        metadata: metadata_str,
    };
    Ok(())
}
