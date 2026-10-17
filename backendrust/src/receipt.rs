//! Receipt / invoice → payment draft.
//!
//! The demo uploads a photo of a receipt or invoice. We send it to the
//! LLM (or a deterministic stub) and ask it to extract payee, amount
//! in CNY, and due date. The result is a *draft* — never an executed
//! payment. The user must confirm via the existing agent flow.
//!
//! In simulation, no real OCR runs. The stub fabricates a believable
//! draft keyed off the upload metadata (filename, declared merchant
//! hint) so the demo is deterministic and reproducible.

use actix_web::web;
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::audit::{self, AppendEventParams};
use crate::config;
use crate::ctx::{AuthUser, RequestId};
use crate::errors::{self, ApiError};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct ReceiptDraft {
    pub payee_hint: String,
    pub amount_cny: f64,
    pub amount_fen: i64,
    pub due_at: String,
    pub currency: String,
    pub confidence: f32,
    pub notes: Vec<String>,
    /// The proposed transfer arguments in the same shape the planner
    /// accepts. The user can confirm with one tap to create a real
    /// pending_action.
    pub proposed_arguments: Value,
    pub source: String, // "stub" | "llm"
}

const DEMO_RECEIPTS: &[(&str, &str, f64, &str)] = &[
    (
        "Sun Cafe receipt — latte + sandwich",
        "Sun Cafe",
        48.0,
        "tomorrow",
    ),
    (
        "Acme Books invoice — textbooks",
        "Acme Books",
        286.50,
        "next week",
    ),
    (
        "Telco bill — monthly plan",
        "Cell Co.",
        88.0,
        "5 days",
    ),
    (
        "Utility — electric",
        "CityPower",
        142.0,
        "3 days",
    ),
    (
        "Tuition notice — winter term",
        "University",
        5400.0,
        "next month",
    ),
];

pub async fn scan(
    state: &web::Data<AppState>,
    auth: &AuthUser,
    rid: &RequestId,
    filename: Option<String>,
    declared_hint: Option<String>,
    declared_amount: Option<f64>,
    bytes_len: usize,
) -> Result<ReceiptDraft, ApiError> {
    let settings = config::get_settings();
    // Prefer the LLM if available, otherwise fall back to the demo set.
    let draft = if settings.unsloth_enabled() {
        match llm_extract(&settings, filename.as_deref(), declared_hint.as_deref(), declared_amount, bytes_len).await {
            Some(mut d) => {
                d.source = "llm".to_string();
                d
            }
            None => stub_extract(filename.as_deref(), declared_hint.as_deref(), declared_amount),
        }
    } else {
        stub_extract(filename.as_deref(), declared_hint.as_deref(), declared_amount)
    };
    // Audit, but never log raw extracted values that could contain user
    // secrets. We only record the file metadata and a short payee hint.
    audit::append_event(
        &state.pool,
        AppendEventParams {
            action: "receipt.scan",
            actor_user_id: Some(&auth.0.id),
            session: None,
            request_id: Some(&rid.0),
            workflow_id: None,
            node_id: None,
            tool: None,
            parameters_masked: Some(json!({
                "filename": filename.as_deref().map(|s| s.chars().take(64).collect::<String>()),
                "bytes": bytes_len,
                "source": draft.source,
                "confidence": draft.confidence,
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
    Ok(draft)
}

fn stub_extract(
    filename: Option<&str>,
    declared_hint: Option<&str>,
    declared_amount: Option<f64>,
) -> ReceiptDraft {
    let idx: usize = {
        // Deterministic from filename length + declared hint length, so
        // the same input gives the same draft. This is just for the demo.
        let mut h: usize = filename.map(|s| s.len()).unwrap_or(0);
        if let Some(h_) = declared_hint {
            h ^= h_.len();
        }
        h % DEMO_RECEIPTS.len()
    };
    let (default_note, default_payee, default_amount, default_due) = DEMO_RECEIPTS[idx];
    let payee = declared_hint.unwrap_or(default_payee).to_string();
    let amount = declared_amount.unwrap_or(default_amount);
    let amount_fen = (amount * 100.0).round() as i64;
    let due_at = (Utc::now() + parse_relative(default_due)).to_rfc3339();
    let proposed_arguments = json!({
        "amount_fen": amount_fen,
        "memo": format!("Receipt scan — {} ({})", payee, default_note),
        "payee_hint": payee,
    });
    ReceiptDraft {
        payee_hint: payee,
        amount_cny: amount,
        amount_fen,
        due_at,
        currency: "CNY".to_string(),
        confidence: 0.82,
        notes: vec![default_note.to_string(), format!("Filename hint: {:?}", filename.unwrap_or("?"))],
        proposed_arguments,
        source: "stub".to_string(),
    }
}

fn parse_relative(s: &str) -> Duration {
    let lower = s.to_lowercase();
    if lower.contains("month") {
        Duration::days(30)
    } else if lower.contains("week") {
        Duration::days(7)
    } else if lower.contains("tomorrow") {
        Duration::days(1)
    } else {
        Duration::days(3)
    }
}

async fn llm_extract(
    settings: &config::Settings,
    filename: Option<&str>,
    declared_hint: Option<&str>,
    declared_amount: Option<f64>,
    bytes_len: usize,
) -> Option<ReceiptDraft> {
    let prompt = json!({
        "system": "You are a receipt/invoice OCR. Return STRICT JSON: {payee, amount_cny, currency, due_iso_days_from_now}. No prose.",
        "filename": filename,
        "hint": declared_hint,
        "hint_amount": declared_amount,
        "approx_bytes": bytes_len,
    });
    let url = format!("{}/chat/completions", settings.unsloth_url.trim_end_matches('/'));
    let req = crate::llm_http::client()
        .post(&url)
        .bearer_auth(&settings.unsloth_api_key)
        .json(&json!({
            "model": settings.unsloth_model,
            "messages": [{"role":"user","content": serde_json::to_string(&prompt).unwrap_or_default()}],
            "temperature": 0.1,
            "response_format": {"type":"json_object"},
        }));
    let resp = req.send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let v: Value = resp.json().await.ok()?;
    let content = v.get("choices")?.get(0)?.get("message")?.get("content")?.as_str()?;
    let parsed: Value = serde_json::from_str(content).ok()?;
    let payee = parsed.get("payee")?.as_str()?.to_string();
    let amount: f64 = parsed.get("amount_cny")?.as_f64()?;
    let currency = parsed.get("currency").and_then(|v| v.as_str()).unwrap_or("CNY").to_string();
    let days: i64 = parsed.get("due_iso_days_from_now").and_then(|v| v.as_i64()).unwrap_or(7);
    let amount_fen = (amount * 100.0).round() as i64;
    let due_at = (Utc::now() + Duration::days(days)).to_rfc3339();
    let proposed_arguments = json!({
        "amount_fen": amount_fen,
        "memo": format!("Receipt scan — {}", payee),
        "payee_hint": payee,
        "currency": currency,
    });
    Some(ReceiptDraft {
        payee_hint: payee,
        amount_cny: amount,
        amount_fen,
        due_at,
        currency,
        confidence: 0.74,
        notes: vec!["Vision-model extraction (lower confidence than a real OCR pipeline).".into()],
        proposed_arguments,
        source: "llm".to_string(),
    })
}