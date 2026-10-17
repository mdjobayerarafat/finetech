//! Append-only audit log with hash chain.

use chrono::{DateTime, TimeZone, Utc};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};

use crate::ids;
use crate::models::{AuditEvent, SessionRow};

pub const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Debug, Default)]
pub struct AppendEventParams<'a> {
    pub action: &'a str,
    pub actor_user_id: Option<&'a str>,
    pub session: Option<&'a SessionRow>,
    pub request_id: Option<&'a str>,
    pub workflow_id: Option<&'a str>,
    pub node_id: Option<&'a str>,
    pub tool: Option<&'a str>,
    pub parameters_masked: Option<Value>,
    pub risk_tier: Option<&'a str>,
    pub policy_reason: Option<&'a str>,
    pub confirmation_status: Option<&'a str>,
    pub mfa_status: Option<&'a str>,
    pub result: &'a str,
    pub error_code: Option<&'a str>,
}

fn ensure_aware(dt: DateTime<Utc>) -> DateTime<Utc> {
    if dt.timestamp() > 0 {
        dt
    } else {
        Utc.timestamp_opt(0, 0).unwrap()
    }
}

fn canonical_event(
    actor_user_id: Option<&str>,
    session_id: Option<&str>,
    request_id: Option<&str>,
    workflow_id: Option<&str>,
    node_id: Option<&str>,
    action: &str,
    tool: Option<&str>,
    parameters_masked: &Value,
    risk_tier: Option<&str>,
    policy_reason: Option<&str>,
    confirmation_status: Option<&str>,
    mfa_status: Option<&str>,
    result: &str,
    error_code: Option<&str>,
    created_at: DateTime<Utc>,
    prev_hash: &str,
) -> String {
    let payload = serde_json::json!({
        "actor_user_id": actor_user_id,
        "session_id": session_id,
        "request_id": request_id,
        "workflow_id": workflow_id,
        "node_id": node_id,
        "action": action,
        "tool": tool,
        "parameters_masked": parameters_masked,
        "risk_tier": risk_tier,
        "policy_reason": policy_reason,
        "confirmation_status": confirmation_status,
        "mfa_status": mfa_status,
        "result": result,
        "error_code": error_code,
        "created_at": created_at.to_rfc3339(),
        "prev_hash": prev_hash,
    });
    serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string())
}

pub async fn last_hash(pool: &PgPool) -> Result<String, sqlx::Error> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT hash FROM audit_events ORDER BY created_at DESC, id DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| r.0).unwrap_or_else(|| GENESIS_HASH.to_string()))
}

pub async fn append_event(pool: &PgPool, p: AppendEventParams<'_>) -> Result<AuditEvent, sqlx::Error> {
    let prev = last_hash(pool).await?;
    let now = Utc::now();
    let masked = p.parameters_masked.unwrap_or(Value::Object(Default::default()));
    let canonical = canonical_event(
        p.actor_user_id,
        p.session.map(|s| s.id.as_str()),
        p.request_id,
        p.workflow_id,
        p.node_id,
        p.action,
        p.tool,
        &masked,
        p.risk_tier,
        p.policy_reason,
        p.confirmation_status,
        p.mfa_status,
        p.result,
        p.error_code,
        now,
        &prev,
    );
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    let digest = hex::encode(hasher.finalize());
    let id = ids::new_id("evt");
    let parameters_str = serde_json::to_string(&masked).unwrap_or("{}".to_string());
    sqlx::query(
        "INSERT INTO audit_events (id, created_at, actor_user_id, session_id, request_id,
         workflow_id, node_id, action, tool, parameters_masked, risk_tier, policy_reason,
         confirmation_status, mfa_status, result, error_code, prev_hash, hash)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)",
    )
    .bind(&id)
    .bind(now)
    .bind(p.actor_user_id)
    .bind(p.session.map(|s| s.id.as_str()))
    .bind(p.request_id)
    .bind(p.workflow_id)
    .bind(p.node_id)
    .bind(p.action)
    .bind(p.tool)
    .bind(parameters_str)
    .bind(p.risk_tier)
    .bind(p.policy_reason)
    .bind(p.confirmation_status)
    .bind(p.mfa_status)
    .bind(p.result)
    .bind(p.error_code)
    .bind(&prev)
    .bind(&digest)
    .execute(pool)
    .await?;

    Ok(AuditEvent {
        id,
        created_at: now,
        actor_user_id: p.actor_user_id.map(|s| s.to_string()),
        session_id: p.session.map(|s| s.id.clone()),
        request_id: p.request_id.map(|s| s.to_string()),
        workflow_id: p.workflow_id.map(|s| s.to_string()),
        node_id: p.node_id.map(|s| s.to_string()),
        action: p.action.to_string(),
        tool: p.tool.map(|s| s.to_string()),
        parameters_masked: serde_json::to_string(&masked).unwrap_or("{}".to_string()),
        risk_tier: p.risk_tier.map(|s| s.to_string()),
        policy_reason: p.policy_reason.map(|s| s.to_string()),
        confirmation_status: p.confirmation_status.map(|s| s.to_string()),
        mfa_status: p.mfa_status.map(|s| s.to_string()),
        result: p.result.to_string(),
        error_code: p.error_code.map(|s| s.to_string()),
        prev_hash: prev,
        hash: digest,
    })
}

#[derive(Debug, Serialize)]
pub struct ChainVerify {
    pub ok: bool,
    pub events: usize,
    pub break_at: Option<String>,
    pub reason: Option<String>,
    pub expected: Option<String>,
    pub actual: Option<String>,
}

pub async fn verify_chain(
    pool: &PgPool,
    from_id: Option<&str>,
    to_id: Option<&str>,
) -> Result<ChainVerify, sqlx::Error> {
    let mut q = String::from(
        "SELECT id, created_at, actor_user_id, session_id, request_id, workflow_id, node_id, action, tool,
                parameters_masked, risk_tier, policy_reason, confirmation_status, mfa_status, result, error_code,
                prev_hash, hash FROM audit_events ORDER BY created_at ASC, id ASC",
    );
    if from_id.is_some() {
        q.push_str(" WHERE id >= ?");
    }
    if to_id.is_some() {
        if from_id.is_some() {
            q.push_str(" AND id <= ?");
        } else {
            q.push_str(" WHERE id <= ?");
        }
    }

    let mut query = sqlx::query(&q);
    if let Some(f) = from_id {
        query = query.bind(f);
    }
    if let Some(t) = to_id {
        query = query.bind(t);
    }
    let rows = query.fetch_all(pool).await?;
    let mut prev = GENESIS_HASH.to_string();
    let mut count = 0usize;
    for row in rows {
        let ev_id: String = row.get("id");
        let prev_hash: String = row.get("prev_hash");
        let hash: String = row.get("hash");
        let created_at: DateTime<Utc> = ensure_aware(row.get("created_at"));
        if prev_hash != prev {
            return Ok(ChainVerify {
                ok: false,
                events: count,
                break_at: Some(ev_id),
                reason: Some("prev_hash mismatch".to_string()),
                expected: Some(prev),
                actual: Some(prev_hash),
            });
        }
        let masked_str: String = row.get("parameters_masked");
        let masked: Value = serde_json::from_str(&masked_str).unwrap_or(Value::Object(Default::default()));
        let canonical = canonical_event(
            row.get::<Option<String>, _>("actor_user_id").as_deref(),
            row.get::<Option<String>, _>("session_id").as_deref(),
            row.get::<Option<String>, _>("request_id").as_deref(),
            row.get::<Option<String>, _>("workflow_id").as_deref(),
            row.get::<Option<String>, _>("node_id").as_deref(),
            row.get::<String, _>("action").as_str(),
            row.get::<Option<String>, _>("tool").as_deref(),
            &masked,
            row.get::<Option<String>, _>("risk_tier").as_deref(),
            row.get::<Option<String>, _>("policy_reason").as_deref(),
            row.get::<Option<String>, _>("confirmation_status").as_deref(),
            row.get::<Option<String>, _>("mfa_status").as_deref(),
            row.get::<String, _>("result").as_str(),
            row.get::<Option<String>, _>("error_code").as_deref(),
            created_at,
            &prev_hash,
        );
        let mut hasher = Sha256::new();
        hasher.update(canonical.as_bytes());
        let digest = hex::encode(hasher.finalize());
        if digest != hash {
            return Ok(ChainVerify {
                ok: false,
                events: count,
                break_at: Some(ev_id),
                reason: Some("hash mismatch".to_string()),
                expected: Some(digest),
                actual: Some(hash),
            });
        }
        prev = hash;
        count += 1;
    }
    Ok(ChainVerify {
        ok: true,
        events: count,
        break_at: None,
        reason: None,
        expected: None,
        actual: None,
    })
}
