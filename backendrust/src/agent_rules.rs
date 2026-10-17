//! Agent rules engine.
//!
//! Admins can register rules that constrain what the agent is allowed to do.
//! Rules are evaluated *server-side* in the planner, so they are enforced
//! even if the LLM is tricked. Every match is recorded to `audit_events`
//! with `action = 'agent_rule.match'` and the rule's id/severity, so the
//! admin console can show "rule X fired N times today".
//!
//! ## Rule kinds
//!
//! | kind                    | what it matches                                    | default severity |
//! |-------------------------|----------------------------------------------------|------------------|
//! | `forbid_tool`           | exact match on the canonical tool name             | block            |
//! | `restrict_payee`        | substring of payee.display_name or phone_tail      | block            |
//! | `restrict_memo`         | substring (case-insensitive) of the memo           | block            |
//! | `restrict_amount_fen`   | single-transfer cap; any amount > N is denied      | block            |
//! | `restrict_daily_fen`    | daily cumulative cap on a single account           | block            |
//! | `system_prompt_prefix`  | injected as a paragraph at the top of system prompt| warn             |
//!
//! ## Severity
//!
//! * `block` — the rule denies the action. The planner returns a
//!   `ToolError::Forbidden` whose message is shown to the user. Audit row
//!   is written.
//! * `warn`  — the rule records an audit row but does *not* block. Useful
//!   for soft system-prompt injections and "see how often this triggers"
//!   observability.
//! * `log`   — purely observational. No audit row is written; the match
//!   is returned to the caller for logging only.
//!
//! ## Applies-to
//!
//! `applies_to` is either `'all'` (any user) or a specific user id. The
//! planner evaluates the union of `applies_to = 'all'` plus the
//! caller's user_id. There is no group/role modelling yet.

use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::collections::HashSet;
use tokio::sync::RwLock;

use crate::audit::{self, AppendEventParams};
use crate::errors::{self, ApiError};
use crate::ids;
use crate::models::User;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AgentRule {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub pattern: String,
    pub severity: String,
    pub applies_to: String,
    pub enabled: bool,
    pub notes: Option<String>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewAgentRule {
    pub kind: String,
    pub name: String,
    pub pattern: String,
    #[serde(default = "default_severity")]
    pub severity: String,
    #[serde(default = "default_applies_to")]
    pub applies_to: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub notes: Option<String>,
}

fn default_severity() -> String {
    "block".to_string()
}
fn default_applies_to() -> String {
    "all".to_string()
}
fn default_enabled() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAgentRule {
    pub name: Option<String>,
    pub pattern: Option<String>,
    pub severity: Option<String>,
    pub applies_to: Option<String>,
    pub enabled: Option<bool>,
    pub notes: Option<String>,
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

const ALLOWED_KINDS: &[&str] = &[
    "forbid_tool",
    "restrict_payee",
    "restrict_memo",
    "restrict_amount_fen",
    "restrict_daily_fen",
    "system_prompt_prefix",
];
const ALLOWED_SEVERITIES: &[&str] = &["block", "warn", "log"];

pub fn validate_rule_kind(kind: &str) -> Result<(), ApiError> {
    if ALLOWED_KINDS.contains(&kind) {
        Ok(())
    } else {
        Err(errors::schema_invalid(&format!(
            "unknown rule kind `{kind}`; allowed: {}",
            ALLOWED_KINDS.join(", ")
        )))
    }
}

pub fn validate_rule_severity(sev: &str) -> Result<(), ApiError> {
    if ALLOWED_SEVERITIES.contains(&sev) {
        Ok(())
    } else {
        Err(errors::schema_invalid(&format!(
            "unknown severity `{sev}`; allowed: {}",
            ALLOWED_SEVERITIES.join(", ")
        )))
    }
}

// ---------------------------------------------------------------------------
// CRUD
// ---------------------------------------------------------------------------

pub async fn list_rules(pool: &PgPool) -> Result<Vec<AgentRule>, sqlx::Error> {
    sqlx::query_as::<_, AgentRule>(
        "SELECT * FROM agent_rules ORDER BY enabled DESC, kind ASC, name ASC",
    )
    .fetch_all(pool)
    .await
}

pub async fn get_rule(pool: &PgPool, id: &str) -> Result<Option<AgentRule>, sqlx::Error> {
    sqlx::query_as::<_, AgentRule>("SELECT * FROM agent_rules WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn create_rule(
    pool: &PgPool,
    admin: &User,
    new_rule: NewAgentRule,
) -> Result<AgentRule, ApiError> {
    validate_rule_kind(&new_rule.kind)?;
    validate_rule_severity(&new_rule.severity)?;
    if new_rule.name.trim().is_empty() {
        return Err(errors::schema_invalid("name is required"));
    }
    if new_rule.pattern.trim().is_empty() {
        return Err(errors::schema_invalid("pattern is required"));
    }
    // Sanity-check the pattern for the amount-cap rules.
    if matches!(
        new_rule.kind.as_str(),
        "restrict_amount_fen" | "restrict_daily_fen"
    ) {
        if new_rule.pattern.parse::<i64>().is_err() {
            return Err(errors::schema_invalid(
                "amount-cap rules require pattern = integer fen",
            ));
        }
    }
    let id = ids::new_id("rule");
    let now = Utc::now();
    sqlx::query(
        "INSERT INTO agent_rules
            (id, kind, name, pattern, severity, applies_to, enabled, notes, created_by, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $10)",
    )
    .bind(&id)
    .bind(&new_rule.kind)
    .bind(&new_rule.name)
    .bind(&new_rule.pattern)
    .bind(&new_rule.severity)
    .bind(&new_rule.applies_to)
    .bind(new_rule.enabled)
    .bind(&new_rule.notes)
    .bind(&admin.id)
    .bind(now)
    .execute(pool)
    .await
    .map_err(|e| {
        log::error!("agent_rules.create failed: {e}");
        errors::internal_error()
    })?;
    audit::append_event(
        pool,
        AppendEventParams {
            action: "agent_rule.create",
            actor_user_id: Some(&admin.id),
            session: None,
            request_id: None,
            workflow_id: None,
            node_id: None,
            tool: None,
            parameters_masked: Some(json!({
                "id": id, "kind": new_rule.kind, "name": new_rule.name,
                "severity": new_rule.severity, "applies_to": new_rule.applies_to,
            })),
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
    invalidate_cache();
    Ok(AgentRule {
        id,
        kind: new_rule.kind,
        name: new_rule.name,
        pattern: new_rule.pattern,
        severity: new_rule.severity,
        applies_to: new_rule.applies_to,
        enabled: new_rule.enabled,
        notes: new_rule.notes,
        created_by: Some(admin.id.clone()),
        created_at: now,
        updated_at: now,
    })
}

pub async fn update_rule(
    pool: &PgPool,
    admin: &User,
    id: &str,
    patch: UpdateAgentRule,
) -> Result<AgentRule, ApiError> {
    let current: Option<AgentRule> = sqlx::query_as("SELECT * FROM agent_rules WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|_| errors::internal_error())?;
    let current = current.ok_or_else(|| errors::not_found("Agent rule not found"))?;
    if let Some(s) = &patch.severity {
        validate_rule_severity(s)?;
    }
    let new_severity = patch.severity.clone().unwrap_or_else(|| current.severity.clone());
    let new_name = patch.name.clone().unwrap_or_else(|| current.name.clone());
    let new_pattern = patch.pattern.clone().unwrap_or_else(|| current.pattern.clone());
    let new_applies = patch.applies_to.clone().unwrap_or_else(|| current.applies_to.clone());
    let new_enabled = patch.enabled.unwrap_or(current.enabled);
    let new_notes = match patch.notes {
        Some(n) => Some(n),
        None => current.notes.clone(),
    };
    sqlx::query(
        "UPDATE agent_rules
         SET name = $2, pattern = $3, severity = $4, applies_to = $5, enabled = $6, notes = $7
         WHERE id = $1",
    )
    .bind(id)
    .bind(&new_name)
    .bind(&new_pattern)
    .bind(&new_severity)
    .bind(&new_applies)
    .bind(new_enabled)
    .bind(&new_notes)
    .execute(pool)
    .await
    .map_err(|_| errors::internal_error())?;
    audit::append_event(
        pool,
        AppendEventParams {
            action: "agent_rule.update",
            actor_user_id: Some(&admin.id),
            session: None,
            request_id: None,
            workflow_id: None,
            node_id: None,
            tool: None,
            parameters_masked: Some(json!({
                "id": id, "name": new_name, "severity": new_severity,
                "enabled": new_enabled, "applies_to": new_applies,
            })),
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
    invalidate_cache();
    get_rule(pool, id)
        .await
        .map_err(|_| errors::internal_error())?
        .ok_or_else(|| errors::not_found("Agent rule not found after update"))
}

pub async fn delete_rule(pool: &PgPool, admin: &User, id: &str) -> Result<(), ApiError> {
    let res = sqlx::query("DELETE FROM agent_rules WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|_| errors::internal_error())?;
    if res.rows_affected() == 0 {
        return Err(errors::not_found("Agent rule not found"));
    }
    audit::append_event(
        pool,
        AppendEventParams {
            action: "agent_rule.delete",
            actor_user_id: Some(&admin.id),
            session: None,
            request_id: None,
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
    invalidate_cache();
    Ok(())
}

// ---------------------------------------------------------------------------
// Cached read for the planner hot path.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuleSnapshot {
    pub rules: Vec<AgentRule>,
    pub fetched_at: DateTime<Utc>,
}

static CACHE: Lazy<RwLock<Option<RuleSnapshot>>> = Lazy::new(|| RwLock::new(None));
const CACHE_TTL_SECS: i64 = 10;

pub async fn snapshot(pool: &PgPool) -> RuleSnapshot {
    {
        let guard = CACHE.read().await;
        if let Some(s) = guard.as_ref() {
            let age = (Utc::now() - s.fetched_at).num_seconds();
            if age < CACHE_TTL_SECS {
                return s.clone();
            }
        }
    }
    let rules = list_rules(pool).await.unwrap_or_default();
    let snap = RuleSnapshot {
        rules,
        fetched_at: Utc::now(),
    };
    {
        let mut guard = CACHE.write().await;
        *guard = Some(snap.clone());
    }
    snap
}

pub fn invalidate_cache() {
    // Best-effort. We use try_write so a planner in flight never blocks
    // an admin's edit; it just sees a slightly stale snapshot.
    if let Ok(mut guard) = CACHE.try_write() {
        *guard = None;
    }
}

fn applies(rule: &AgentRule, user_id: &str) -> bool {
    rule.enabled && (rule.applies_to == "all" || rule.applies_to == user_id)
}

// ---------------------------------------------------------------------------
// Evaluation entry points (used by the planner + system prompt).
// ---------------------------------------------------------------------------

/// True if the tool is forbidden for this user by any enabled `forbid_tool`
/// rule. The planner must remove the tool from the allowlist.
pub async fn is_tool_forbidden(
    pool: &PgPool,
    user_id: &str,
    tool: &str,
) -> Result<Option<AgentRule>, sqlx::Error> {
    let snap = snapshot(pool).await;
    Ok(snap
        .rules
        .into_iter()
        .find(|r| applies(r, user_id)
            && r.kind == "forbid_tool"
            && r.pattern == tool
            && r.severity == "block"))
}

/// All `forbid_tool` rules currently in effect (for the planner allowlist
/// and the admin UI).
pub async fn forbidden_tools(pool: &PgPool, user_id: &str) -> Vec<AgentRule> {
    let snap = snapshot(pool).await;
    snap.rules
        .into_iter()
        .filter(|r| {
            applies(r, user_id)
                && r.kind == "forbid_tool"
                && r.severity == "block"
        })
        .collect()
}

#[derive(Debug, Clone, Serialize)]
pub struct RuleHit {
    pub rule: AgentRule,
    pub matched_field: String,
    pub matched_value: String,
}

pub struct TransferContext<'a> {
    pub payee_name: &'a str,
    pub payee_phone_tail: Option<&'a str>,
    pub memo: &'a str,
    pub amount_fen: i64,
    pub source_account_id: &'a str,
}

/// Evaluate a transfer against all enabled rules. Returns the *first* hard
/// block, plus a list of all warn/log hits for observability.
pub async fn evaluate_transfer(
    pool: &PgPool,
    user_id: &str,
    ctx: &TransferContext<'_>,
) -> Result<EvaluationOutcome, sqlx::Error> {
    let snap = snapshot(pool).await;
    let mut blocks: Vec<RuleHit> = vec![];
    let mut warns: Vec<RuleHit> = vec![];
    for r in snap.rules.iter() {
        if !applies(r, user_id) {
            continue;
        }
        match r.kind.as_str() {
            "restrict_payee" => {
                let pat = r.pattern.to_lowercase();
                let name_hit = ctx.payee_name.to_lowercase().contains(&pat);
                let phone_hit = ctx
                    .payee_phone_tail
                    .map(|p| p.contains(r.pattern.as_str()))
                    .unwrap_or(false);
                if name_hit || phone_hit {
                    let val = if name_hit {
                        ctx.payee_name.to_string()
                    } else {
                        ctx.payee_phone_tail.unwrap_or("").to_string()
                    };
                    let hit = RuleHit {
                        rule: r.clone(),
                        matched_field: if name_hit { "payee_name" } else { "phone_tail" }.to_string(),
                        matched_value: val,
                    };
                    match r.severity.as_str() {
                        "block" => blocks.push(hit),
                        _ => warns.push(hit),
                    }
                }
            }
            "restrict_memo" => {
                if ctx.memo.to_lowercase().contains(&r.pattern.to_lowercase()) {
                    let hit = RuleHit {
                        rule: r.clone(),
                        matched_field: "memo".to_string(),
                        matched_value: ctx.memo.to_string(),
                    };
                    match r.severity.as_str() {
                        "block" => blocks.push(hit),
                        _ => warns.push(hit),
                    }
                }
            }
            "restrict_amount_fen" => {
                if let Ok(cap) = r.pattern.parse::<i64>() {
                    if ctx.amount_fen > cap {
                        let hit = RuleHit {
                            rule: r.clone(),
                            matched_field: "amount_fen".to_string(),
                            matched_value: ctx.amount_fen.to_string(),
                        };
                        match r.severity.as_str() {
                            "block" => blocks.push(hit),
                            _ => warns.push(hit),
                        }
                    }
                }
            }
            "restrict_daily_fen" => {
                if let Ok(cap) = r.pattern.parse::<i64>() {
                    let today = crate::policy::today_total_fen(pool, ctx.source_account_id)
                        .await
                        .unwrap_or(0);
                    if today + ctx.amount_fen > cap {
                        let hit = RuleHit {
                            rule: r.clone(),
                            matched_field: "daily_total_fen".to_string(),
                            matched_value: (today + ctx.amount_fen).to_string(),
                        };
                        match r.severity.as_str() {
                            "block" => blocks.push(hit),
                            _ => warns.push(hit),
                        }
                    }
                }
            }
            _ => {}
        }
    }
    Ok(EvaluationOutcome { blocks, warns })
}

#[derive(Debug, Clone, Serialize)]
pub struct EvaluationOutcome {
    pub blocks: Vec<RuleHit>,
    pub warns: Vec<RuleHit>,
}

impl EvaluationOutcome {
    pub fn blocked(&self) -> bool {
        !self.blocks.is_empty()
    }
    pub fn block_reason(&self) -> Option<String> {
        self.blocks.first().map(|h| {
            format!(
                "admin rule `{}` ({}) blocked this action: matched `{}` on `{}`",
                h.rule.name,
                h.rule.kind,
                h.matched_value,
                h.matched_field
            )
        })
    }
}

/// Build the system-prompt paragraph contributed by `system_prompt_prefix`
/// rules. Returns a single string, ready to splice into the LLM prompt.
pub async fn system_prompt_prefix(pool: &PgPool, user_id: &str) -> String {
    let snap = snapshot(pool).await;
    let mut paragraphs: Vec<String> = vec![];
    for r in snap.rules.iter() {
        if !applies(r, user_id) || r.kind != "system_prompt_prefix" {
            continue;
        }
        if r.pattern.trim().is_empty() {
            continue;
        }
        let prefix = if r.severity == "block" {
            "ADMIN POLICY (MUST OBEY):"
        } else {
            "ADMIN GUIDANCE (soft, follow when possible):"
        };
        paragraphs.push(format!("- [{}] {} — {}", prefix, r.name, r.pattern));
    }
    if paragraphs.is_empty() {
        String::new()
    } else {
        format!(
            "\n## Admin-configured rules\nThese rules were set by an administrator and take precedence over user requests. Do not invent ways to bypass them.\n\n{}",
            paragraphs.join("\n")
        )
    }
}

/// Emit one audit row per block+warn hit. Best-effort: failures are
/// logged but never break the request.
pub async fn record_hits(
    pool: &PgPool,
    user_id: &str,
    request_id: Option<&str>,
    outcome: &EvaluationOutcome,
) {
    let mut seen: HashSet<String> = HashSet::new();
    for hit in outcome.blocks.iter().chain(outcome.warns.iter()) {
        if !seen.insert(hit.rule.id.clone()) {
            continue;
        }
        audit::append_event(
            pool,
            AppendEventParams {
                action: "agent_rule.match",
                actor_user_id: Some(user_id),
                session: None,
                request_id,
                workflow_id: None,
                node_id: None,
                tool: None,
                parameters_masked: Some(json!({
                    "rule_id": hit.rule.id,
                    "rule_name": hit.rule.name,
                    "kind": hit.rule.kind,
                    "severity": hit.rule.severity,
                    "matched_field": hit.matched_field,
                    "matched_value": hit.matched_value,
                })),
                risk_tier: Some(if hit.rule.severity == "block" { "Red" } else { "Yellow" }),
                policy_reason: Some(&hit.rule.name),
                confirmation_status: None,
                mfa_status: None,
                result: if hit.rule.severity == "block" { "blocked" } else { "warned" },
                error_code: None,
            },
        )
        .await
        .ok();
    }
}

/// Tiny helper: turn a list of `AgentRule`s into the JSON shape the
/// planner prompt uses.
pub fn snapshot_to_json(snap: &RuleSnapshot) -> Value {
    json!({
        "rules": snap.rules,
    })
}
