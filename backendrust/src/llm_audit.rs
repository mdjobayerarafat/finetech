//! LLM call audit log — every call to the LLM (or stub) writes one row.
//!
//! Wraps `llm::propose_async` with timing and outcome recording. The schema
//! is intentionally simple: provider, model, duration, success, intent,
//! fallback flag. We compute a stable `prompt_fingerprint` (sha256 of the
//! canonical prompt) so admin can later dedupe repeat calls.

use chrono::Utc;
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::PgPool;

use crate::config::Settings;
use crate::ids;
use crate::llm::{self, Proposal};
use crate::llm_http;

#[derive(Debug, Clone, Default)]
pub struct AuditContext<'a> {
    pub user_id: Option<&'a str>,
    pub request_id: Option<&'a str>,
    pub conversation_id: Option<&'a str>,
}

/// Top-level entrypoint: propose a plan and write one audit row. The route
/// can use this in place of `llm::propose` to gain observability for free.
pub async fn audited_propose(
    pool: &PgPool,
    settings: &Settings,
    ctx: AuditContext<'_>,
    user_message: &str,
    state_summary: &Value,
    language_hint: Option<&str>,
    admin_prefix: &str,
) -> Proposal {
    let started = std::time::Instant::now();
    let prompt_fp = fingerprint_prompt(user_message, state_summary, language_hint);
    let provider = if settings.unsloth_enabled() {
        "unsloth"
    } else {
        "stub"
    };
    let model = if settings.unsloth_enabled() {
        settings.unsloth_model.clone()
    } else {
        "regex-stub".to_string()
    };
    // Try Unsloth first when enabled.
    if settings.unsloth_enabled() {
        match llm_http::propose_unsloth(settings, user_message, state_summary, language_hint, admin_prefix).await {
            Some(p) => {
                let duration_ms = started.elapsed().as_millis() as i32;
                write_row(
                    pool,
                    ctx,
                    provider,
                    &model,
                    &prompt_fp,
                    duration_ms,
                    Some(p.intent.as_str()),
                    true,
                    None,
                    false,
                )
                .await
                .ok();
                return p;
            }
            None => {
                // Fall through to the stub. Mark the stub row as
                // `used_fallback = true` so the admin console can show a
                // "LLM unreachable" warning.
                let p = llm::stub_propose(user_message, state_summary, language_hint);
                let duration_ms = started.elapsed().as_millis() as i32;
                write_row(
                    pool,
                    ctx,
                    "stub",
                    "regex-stub",
                    &prompt_fp,
                    duration_ms,
                    Some(p.intent.as_str()),
                    true,
                    None,
                    true,
                )
                .await
                .ok();
                return p;
            }
        }
    }
    let p = llm::stub_propose(user_message, state_summary, language_hint);
    let duration_ms = started.elapsed().as_millis() as i32;
    write_row(
        pool,
        ctx,
        "stub",
        "regex-stub",
        &prompt_fp,
        duration_ms,
        Some(p.intent.as_str()),
        true,
        None,
        false,
    )
    .await
    .ok();
    p
}

#[allow(clippy::too_many_arguments)]
async fn write_row(
    pool: &PgPool,
    ctx: AuditContext<'_>,
    provider: &str,
    model: &str,
    prompt_fp: &str,
    duration_ms: i32,
    intent: Option<&str>,
    success: bool,
    error_message: Option<&str>,
    used_fallback: bool,
) -> Result<(), sqlx::Error> {
    let id = ids::new_id("lc");
    sqlx::query(
        "INSERT INTO llm_call_audit (id, created_at, actor_user_id, request_id, conversation_id, provider, model, prompt_fingerprint, duration_ms, intent, success, error_message, used_fallback) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
    )
    .bind(&id)
    .bind(Utc::now())
    .bind(ctx.user_id)
    .bind(ctx.request_id)
    .bind(ctx.conversation_id)
    .bind(provider)
    .bind(model)
    .bind(prompt_fp)
    .bind(duration_ms)
    .bind(intent)
    .bind(success)
    .bind(error_message)
    .bind(used_fallback)
    .execute(pool)
    .await?;
    Ok(())
}

/// Stable hash of the user message + state. We do *not* include language
/// hint or the system prompt so the same question for the same wallet
/// state has the same fingerprint across requests.
pub fn fingerprint_prompt(text: &str, state: &Value, lang: Option<&str>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hasher.update(b"|");
    hasher.update(
        serde_json::to_string(state)
            .unwrap_or_default()
            .as_bytes(),
    );
    hasher.update(b"|");
    if let Some(l) = lang {
        hasher.update(l.as_bytes());
    }
    hex::encode(hasher.finalize())
}
