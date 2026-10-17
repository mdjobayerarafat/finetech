//! Smart transfers as an AI agent — payee management, fuzzy resolve,
//! scheduled transfers, split-bill collection, and an LLM-narrated
//! security review for every draft.
//!
//! The agent (chat) already calls into the planner to build transfers.
//! This module is the *programmatic* surface that the frontend's
//! `/transfers` page uses directly: add a beneficial account by name
//! or phone tail, resolve "Li Wei" or "13800001234" to a payee, ask
//! the LLM to review a draft, and run split-bill collection.

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::PgPool;

use crate::config;
use crate::ids;
use crate::models::{Payee, SplitBill, SplitBillPart, User};
use crate::planner;
use crate::tools::ToolError;

#[derive(Debug, Clone, Serialize)]
pub struct PayeeSuggestion {
    pub payee: Payee,
    pub match_kind: String, // "exact_name" | "phone_tail" | "prefix_name" | "contains_name"
    pub confidence: f32,
    /// A short, plain-language note the AI agent can speak or display.
    pub note: String,
}

/// Fuzzy resolve a payee by name, phone tail, or note text. Returns up
/// to N ranked matches. The frontend uses the top match by default and
/// falls back to a "create new payee" form for the rest.
pub async fn suggest_payees(
    pool: &PgPool,
    user: &User,
    query: &str,
    limit: i64,
) -> Result<Vec<PayeeSuggestion>, sqlx::Error> {
    let q = query.trim();
    if q.is_empty() {
        return Ok(vec![]);
    }
    let q_lower = q.to_lowercase();
    let rows: Vec<Payee> = sqlx::query_as(
        "SELECT * FROM payees WHERE owner_user_id = $1 ORDER BY verified DESC, display_name ASC",
    )
    .bind(&user.id)
    .fetch_all(pool)
    .await?;
    let mut scored: Vec<PayeeSuggestion> = Vec::new();
    for p in rows {
        let name = p.display_name.to_lowercase();
        let phone = p.phone_tail.as_deref().unwrap_or("");
        if name == q_lower {
            scored.push(PayeeSuggestion {
                payee: p,
                match_kind: "exact_name".into(),
                confidence: 1.0,
                note: format!("Exact match for {}.", q),
            });
            continue;
        }
        if phone == q_lower {
            scored.push(PayeeSuggestion {
                payee: p,
                match_kind: "phone_tail".into(),
                confidence: 0.98,
                note: format!("Phone tail matches {}.", q),
            });
            continue;
        }
        if name.starts_with(&q_lower) {
            scored.push(PayeeSuggestion {
                payee: p,
                match_kind: "prefix_name".into(),
                confidence: 0.85,
                note: format!("Starts with {}.", q),
            });
            continue;
        }
        if name.contains(&q_lower) || q_lower.contains(&name) {
            scored.push(PayeeSuggestion {
                payee: p,
                match_kind: "contains_name".into(),
                confidence: 0.6,
                note: format!("Contains {}.", q),
            });
            continue;
        }
    }
    scored.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit.max(1) as usize);
    Ok(scored)
}

/// Resolve a single payee for a smart-transfer draft. Used by the
/// frontend's "Pay X" form. Returns the resolved payee or a list of
/// candidates when ambiguous.
pub async fn resolve(
    pool: &PgPool,
    user: &User,
    query: &str,
) -> Result<ResolveResult, sqlx::Error> {
    let suggestions = suggest_payees(pool, user, query, 5).await?;
    if suggestions.is_empty() {
        return Ok(ResolveResult {
            status: "none".into(),
            payee: None,
            candidates: vec![],
            note: format!("No saved payee matches {:?}. Add a beneficial account first.", query),
        });
    }
    let top = suggestions[0].clone();
    let top_confidence = top.confidence;
    if suggestions.len() == 1 || top_confidence >= 0.95 {
        return Ok(ResolveResult {
            status: "resolved".into(),
            payee: Some(top.payee.clone()),
            candidates: suggestions,
            note: top.note.clone(),
        });
    }
    let count = suggestions.len();
    Ok(ResolveResult {
        status: "ambiguous".into(),
        payee: None,
        candidates: suggestions,
        note: format!("{} candidates match {:?}.", count, query),
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct ResolveResult {
    pub status: String, // "resolved" | "ambiguous" | "none"
    pub payee: Option<Payee>,
    pub candidates: Vec<PayeeSuggestion>,
    pub note: String,
}

/// Parse a free-form transfer request. The stub is deterministic; the
/// LLM path is used when a provider is configured. Output is the same
/// shape either way so the frontend can render without branching.
pub async fn parse_request(
    pool: &PgPool,
    settings: &config::Settings,
    user: &User,
    text: &str,
) -> Value {
    if settings.unsloth_enabled() {
        if let Some(v) = llm_parse(pool, settings, user, text).await {
            return v;
        }
    }
    stub_parse(pool, user, text).await
}

async fn llm_parse(
    pool: &PgPool,
    settings: &config::Settings,
    user: &User,
    text: &str,
) -> Option<Value> {
    let url = format!("{}/chat/completions", settings.unsloth_url.trim_end_matches('/'));
    let accounts_summary: Vec<Value> = sqlx::query_as::<_, crate::models::Account>(
        "SELECT * FROM accounts WHERE user_id = $1 ORDER BY created_at ASC",
    )
    .bind(&user.id)
    .fetch_all(pool)
    .await
    .ok()?
    .into_iter()
    .map(|a| {
        json!({
            "id": a.id,
            "display_name": a.display_name,
            "available_balance_fen": a.available_balance_fen,
        })
    })
    .collect();
    let payload = json!({
        "system": "You are BankPilot's smart-transfer parser. Given a user message and a list of their accounts, return STRICT JSON: {amount_fen: int|0, recipient_query: string, memo: string, schedule_iso: string|null, split_participants: string[]|null, needs_clarification: bool, questions: string[]}. Never invent values not implied by the text.",
        "accounts": accounts_summary,
        "text": text,
    });
    let body = serde_json::to_string(&payload).ok()?;
    let req = crate::llm_http::client()
        .post(&url)
        .bearer_auth(&settings.unsloth_api_key)
        .json(&json!({
            "model": settings.unsloth_model,
            "messages": [{"role":"user","content": body}],
            "temperature": 0.2,
            "response_format": {"type":"json_object"},
        }));
    let resp = req.send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let v: Value = resp.json().await.ok()?;
    let content = v.get("choices")?.get(0)?.get("message")?.get("content")?.as_str()?;
    let parsed: Value = serde_json::from_str(content).ok()?;
    let resolved = resolve(pool, user, parsed.get("recipient_query").and_then(|v| v.as_str()).unwrap_or("")).await.ok();
    let mut out = parsed;
    if let Some(rr) = resolved {
        if let Value::Object(ref mut m) = out {
            m.insert(
                "resolved_payee".into(),
                serde_json::to_value(&rr.payee).unwrap_or(Value::Null),
            );
            m.insert("resolve_status".into(), Value::String(rr.status));
            m.insert("candidates".into(), serde_json::to_value(&rr.candidates).unwrap_or(json!([])));
        }
    }
    Some(out)
}

async fn stub_parse(pool: &PgPool, user: &User, text: &str) -> Value {
    let lower = text.to_lowercase();
    // Amount — look for "NNN yuan" / "NNN 元" / "NNN RMB" / "NNN" near verbs.
    let amount_fen: i64 = {
        let re_amount = regex_amount();
        if let Some(c) = re_amount.captures(text) {
            let whole: i64 = c.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
            let cents_str = c.get(2).map(|m| m.as_str()).unwrap_or("");
            let cents: f64 = if cents_str.is_empty() { 0.0 } else { cents_str.parse().unwrap_or(0.0) };
            (whole * 100 + (cents * 100.0).round() as i64).max(0)
        } else {
            0
        }
    };
    // Recipient — try the first named token, with Chinese and English hints.
    let recipient_query = {
        // Heuristics: words after "to"/"给"/"给".
        let mut name = String::new();
        for kw in &["pay ", "to ", "send to ", "给", "转给"] {
            if let Some(idx) = lower.find(kw) {
                let start = idx + kw.len();
                let rest = &text[start..];
                let stop = rest
                    .find(|c: char| c == ' ' || c == ',' || c == '。' || c == '，' || c == '\n')
                    .unwrap_or(rest.len());
                let candidate = rest[..stop].trim();
                if !candidate.is_empty() {
                    name = candidate.to_string();
                    break;
                }
            }
        }
        if name.is_empty() {
            // Fallback: pick the longest Chinese name or the first capitalized token.
            for token in text.split_whitespace() {
                if token.chars().any(|c| matches!(c, '\u{4e00}'..='\u{9fff}')) && token.len() > 1 {
                    name = token.to_string();
                    break;
                }
            }
            if name.is_empty() {
                for token in text.split_whitespace() {
                    if token.chars().next().map(|c| c.is_ascii_uppercase()).unwrap_or(false) {
                        name = token.to_string();
                        break;
                    }
                }
            }
        }
        if name.is_empty() {
            String::new()
        } else {
            name
        }
    };
    // Schedule — look for "tomorrow" / "next monday" / ISO date.
    let schedule_iso: Option<String> = if let Some(c) = regex_relative_date().captures(&lower) {
        let s = c.get(1).map(|m| m.as_str()).unwrap_or("");
        let days = match s {
            "tomorrow" | "明天" => 1,
            "tonight" | "今晚" => 0,
            d if d.contains("day") => 3,
            d if d.contains("week") => 7,
            d if d.contains("month") => 30,
            _ => 7,
        };
        Some((Utc::now() + chrono::Duration::days(days)).to_rfc3339())
    } else {
        None
    };
    // Split — look for "split" / "均摊" / "分摊".
    let wants_split = lower.contains("split") || text.contains("均摊") || text.contains("分摊");
    // Memo — take anything after "for" or "因为".
    let memo = {
        let mut m = String::new();
        for kw in &[" for ", "memo:", "因为", "——"] {
            if let Some(idx) = lower.find(kw) {
                let start = idx + kw.len();
                m = text[start..].trim().to_string();
                if m.len() > 200 {
                    m.truncate(200);
                }
                break;
            }
        }
        m
    };
    // Resolve the recipient against the user's payee book.
    let resolved = resolve(pool, user, &recipient_query).await.ok();
    let mut out = json!({
        "amount_fen": amount_fen,
        "recipient_query": recipient_query,
        "memo": memo,
        "schedule_iso": schedule_iso,
        "split_participants": if wants_split { Value::Array(vec![Value::String("them".into()), Value::String("you".into())]) } else { Value::Null },
        "needs_clarification": amount_fen == 0 || recipient_query.is_empty(),
        "questions": vec![
            if amount_fen == 0 { Some("How much (in CNY)?".to_string()) } else { None },
            if recipient_query.is_empty() { Some("Who should I pay?".to_string()) } else { None },
        ].into_iter().filter_map(|x| x).collect::<Vec<_>>(),
    });
    if let Some(rr) = resolved {
        if let Value::Object(ref mut m) = out {
            m.insert(
                "resolved_payee".into(),
                serde_json::to_value(&rr.payee).unwrap_or(Value::Null),
            );
            m.insert("resolve_status".into(), Value::String(rr.status));
            m.insert("candidates".into(), serde_json::to_value(&rr.candidates).unwrap_or(json!([])));
            m.insert("resolve_note".into(), Value::String(rr.note));
        }
    }
    out
}

fn regex_amount() -> &'static regex::Regex {
    use once_cell::sync::OnceCell;
    static R: OnceCell<regex::Regex> = OnceCell::new();
    R.get_or_init(|| {
        regex::Regex::new(r"(?i)(\d{1,9})(?:[.,](\d{1,2}))?(?:\s*(?:元|块|块钱|rmb|¥|￥|yuan))?").unwrap()
    })
}
fn regex_relative_date() -> &'static regex::Regex {
    use once_cell::sync::OnceCell;
    static R: OnceCell<regex::Regex> = OnceCell::new();
    R.get_or_init(|| regex::Regex::new(r"(?i)\b(tomorrow|tonight|next\s+(?:day|week|month))\b|明天|今晚|下周|下月").unwrap())
}

/// Build a "security review" for a draft. The LLM narrates the
/// combination of fraud guard + policy tier + memo hygiene. Falls
/// back to a templated review if no LLM is reachable.
pub async fn security_review(
    pool: &PgPool,
    settings: &config::Settings,
    user: &User,
    payee: &Payee,
    amount_fen: i64,
    memo: &str,
    schedule: Option<DateTime<Utc>>,
) -> SecurityReview {
    let assessment = crate::fraud::score_transfer(pool, user, payee, amount_fen, memo)
        .await
        .unwrap_or(crate::fraud::FraudAssessment {
            flags: vec![],
            score: 0,
            templated: "no signal".into(),
            narrative: None,
        });
    let injection_phrase = crate::policy::detect_injection(memo);
    let urgency = crate::fraud::detect_urgency(memo).is_some();
    let amount_cny = amount_fen as f64 / 100.0;
    let tier_recommendation = if amount_cny >= 5000.0
        || urgency
        || assessment.flags.iter().any(|f| matches!(f.code.as_str(), "urgency_words" | "payee_flagged"))
    {
        "Red"
    } else if !payee.verified
        || payee.first_used_at.is_none()
        || amount_cny >= 500.0
        || assessment.score >= 25
    {
        "Yellow"
    } else {
        "Yellow" // Single-tap confirmation for non-trivial transfers; the demo
                  // surface is too small to use Green for money movement.
    };
    let schedule_note = schedule
        .map(|d| format!("Scheduled for {}.", d.format("%Y-%m-%d %H:%M UTC")))
        .unwrap_or_else(|| "Runs immediately when confirmed.".to_string());
    let templated = format!(
        "{} is {}{}. {}{} {}",
        payee.display_name,
        if payee.verified { "verified" } else { "not on your verified list" },
        if payee.first_used_at.is_none() { " (you have not paid them before)" } else { "" },
        if urgency { "Memo contains urgency wording — typical scam tell. " } else { "" },
        if injection_phrase.is_some() { "Memo contains a prompt-injection pattern and will be blocked. " } else { "" },
        schedule_note,
    );
    let narrative = if settings.unsloth_enabled() {
        crate::fraud::narrate_scam_flags(settings, &assessment, &payee.display_name, amount_fen).await
    } else {
        None
    };
    SecurityReview {
        recommended_tier: tier_recommendation.into(),
        fraud_score: assessment.score,
        flags: assessment.flags,
        injection_phrase,
        urgency_detected: urgency,
        summary: narrative.unwrap_or(templated),
        schedule_note,
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SecurityReview {
    pub recommended_tier: String,
    pub fraud_score: u8,
    pub flags: Vec<crate::fraud::FraudFlag>,
    pub injection_phrase: Option<String>,
    pub urgency_detected: bool,
    pub summary: String,
    pub schedule_note: String,
}

// --- Split-bill CRUD -------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct SplitBillView {
    #[serde(flatten)]
    pub bill: SplitBill,
    pub parts: Vec<SplitBillPart>,
    pub paid_fen: i64,
    pub remaining_fen: i64,
}

pub async fn create_split_bill(
    pool: &PgPool,
    user: &User,
    title: &str,
    memo: &str,
    total_fen: i64,
    currency: &str,
    source_account_id: Option<&str>,
    due_at: Option<DateTime<Utc>>,
    parts: Vec<NewPart>,
) -> Result<SplitBill, sqlx::Error> {
    if total_fen <= 0 {
        return Err(sqlx::Error::Protocol("total_fen must be > 0".into()));
    }
    let sum: i64 = parts.iter().map(|p| p.share_fen).sum();
    if sum != total_fen {
        return Err(sqlx::Error::Protocol(format!(
            "parts sum to {} fen but total is {} fen",
            sum, total_fen
        )));
    }
    let id = ids::new_id("split");
    let now = Utc::now();
    sqlx::query(
        "INSERT INTO split_bills (id, owner_user_id, title, memo, total_fen, currency, source_account_id, status, due_at, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, 'open', $8, $9, $10)",
    )
    .bind(&id)
    .bind(&user.id)
    .bind(title)
    .bind(memo)
    .bind(total_fen)
    .bind(currency)
    .bind(source_account_id)
    .bind(due_at)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;
    for p in &parts {
        let part_id = ids::new_id("spart");
        sqlx::query(
            "INSERT INTO split_bill_parts (id, split_bill_id, payee_id, participant_name, participant_phone_tail, share_fen, status, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, 'unpaid', $7)",
        )
        .bind(&part_id)
        .bind(&id)
        .bind(p.payee_id.as_deref())
        .bind(&p.participant_name)
        .bind(p.participant_phone_tail.as_deref())
        .bind(p.share_fen)
        .bind(now)
        .execute(pool)
        .await?;
    }
    let bill: SplitBill = sqlx::query_as("SELECT * FROM split_bills WHERE id = $1")
        .bind(&id)
        .fetch_one(pool)
        .await?;
    Ok(bill)
}

#[derive(Debug, Clone)]
pub struct NewPart {
    pub payee_id: Option<String>,
    pub participant_name: String,
    pub participant_phone_tail: Option<String>,
    pub share_fen: i64,
}

pub async fn list_split_bills(pool: &PgPool, user: &User) -> Result<Vec<SplitBillView>, sqlx::Error> {
    let bills: Vec<SplitBill> = sqlx::query_as(
        "SELECT * FROM split_bills WHERE owner_user_id = $1 ORDER BY created_at DESC",
    )
    .bind(&user.id)
    .fetch_all(pool)
    .await?;
    let mut out: Vec<SplitBillView> = Vec::new();
    for b in bills {
        let parts: Vec<SplitBillPart> = sqlx::query_as(
            "SELECT * FROM split_bill_parts WHERE split_bill_id = $1 ORDER BY created_at ASC",
        )
        .bind(&b.id)
        .fetch_all(pool)
        .await?;
        let paid: i64 = parts.iter().filter(|p| p.status == "paid").map(|p| p.share_fen).sum();
        out.push(SplitBillView {
            bill: b.clone(),
            remaining_fen: (b.total_fen - paid).max(0),
            paid_fen: paid,
            parts,
        });
    }
    Ok(out)
}

/// Mark one part as paid by attaching an existing transfer id. The
/// caller (route) is expected to have already executed the transfer
/// via the existing `transfer.create_pending` tool; we only record the
/// link here so the split-bill UI can render the state.
pub async fn mark_part_paid(
    pool: &PgPool,
    user: &User,
    part_id: &str,
    transfer_id: &str,
) -> Result<SplitBillPart, sqlx::Error> {
    // Verify the part belongs to a bill owned by this user.
    let part: SplitBillPart = sqlx::query_as(
        "SELECT * FROM split_bill_parts WHERE id = $1",
    )
    .bind(part_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| sqlx::Error::RowNotFound)?;
    let bill: SplitBill = sqlx::query_as("SELECT * FROM split_bills WHERE id = $1")
        .bind(&part.split_bill_id)
        .fetch_one(pool)
        .await?;
    if bill.owner_user_id != user.id {
        return Err(sqlx::Error::RowNotFound);
    }
    let now = Utc::now();
    let updated: SplitBillPart = sqlx::query_as(
        "UPDATE split_bill_parts SET status = 'paid', paid_at = $1, paid_transfer_id = $2 WHERE id = $3 RETURNING *",
    )
    .bind(now)
    .bind(transfer_id)
    .bind(part_id)
    .fetch_one(pool)
    .await?;
    Ok(updated)
}

#[allow(dead_code)]
fn _silence() {
    let _ = planner::resolve_account;
}
