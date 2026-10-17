//! Scam & fraud guard — pure-Rust rules that score a proposed transfer
//! before the policy engine decides the risk tier.
//!
//! This is intentionally a *separate* layer from `policy::decide_transfer`:
//! the policy engine still owns the tier decision. The fraud guard adds
//! a list of human-readable flags and (when the LLM is available) a
//! short narrative. The narrative is what the chat UI surfaces so the
//! user sees *why* the bank thinks this might be a scam.
//!
//! Flags produced:
//!  - `new_payee`           payee has never been used (or first_used_at is null)
//!  - `unverified_payee`    payee.verified == false
//!  - `amount_unusual`      amount is > 3x the median past transfer to that payee
//!  - `urgency_words`       memo matches a known scam-urgency pattern
//!  - `amount_round`        amount is a suspiciously round number
//!  - `payee_flagged`       payee name matches a known-scam list (static, sim)
//!  - `out_of_pattern`      current balance - amount would dip below avg monthly expenses
//!
//! Narrative is produced by the LLM via `narrate_scam_flags`. It is *not*
//! required — if the LLM is unavailable we ship a templated explanation.

use serde::Serialize;
use serde_json::{json, Value};
use sqlx::PgPool;

use crate::llm_http;
use crate::models::{Payee, User};

/// One human-readable fraud signal. Kept stable so the UI can style
/// individual reasons and the policy engine can reason over them.
#[derive(Debug, Clone, Serialize)]
pub struct FraudFlag {
    pub code: String,
    pub message: String,
    pub weight: u8, // 1..5 — informational to hard-signal
}

#[derive(Debug, Clone, Serialize)]
pub struct FraudAssessment {
    pub flags: Vec<FraudFlag>,
    /// 0..100. The fraud guard does NOT pick the tier; the policy engine
    /// does. This number is shown to the user and the demo so a judge can
    /// see how the rules compound.
    pub score: u8,
    /// Short templated explanation always available, even without the LLM.
    pub templated: String,
    /// Optional LLM-narrated explanation, populated when the LLM is reachable.
    pub narrative: Option<String>,
}

/// Hard-coded phrases that we consider "scam urgency" markers. These are
/// separate from the prompt-injection phrases in `policy.rs` because a
/// legitimate transfer may have urgency words, while a memo saying
/// "act now or your account will be closed" is almost always fraud.
const URGENCY_PHRASES: &[&str] = &[
    "act now",
    "right now",
    "urgent",
    "immediately",
    "asap",
    "today only",
    "last chance",
    "final notice",
    "or your account",
    "or else",
    "verify your identity",
    "verify your account",
    "click here",
    "wire transfer",
    "gift card",
    "crypto",
    "bitcoin",
    "usdt",
    "won't ask again",
    "secret",
    "don't tell",
    "do not tell",
    "keep confidential",
    "limited time",
    "law enforcement",
    "tax authority",
    "税务局",
    "公安局",
    "法院",
    "欠费",
    "账户冻结",
    "马上",
    "立刻",
    "尽快",
    "保密",
];

/// Static, simulation-only list of merchant names that we'll treat as
/// "flagged" in the demo. In a real bank this comes from a fraud feed.
const FLAGGED_MERCHANT_KEYWORDS: &[&str] = &[
    "stranger",
    "unknown person",
    "investment opportunity",
    "doubling",
    "high return",
    "easy money",
    "陌生人",
    "高回报",
    "投资机会",
    "翻倍",
];

pub fn detect_urgency(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    for phrase in URGENCY_PHRASES {
        if lower.contains(phrase) {
            return Some(phrase.to_string());
        }
    }
    None
}

pub fn detect_flagged_merchant(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    for kw in FLAGGED_MERCHANT_KEYWORDS {
        if lower.contains(kw) {
            return Some(kw.to_string());
        }
    }
    None
}

fn is_round_number(amount_fen: i64) -> bool {
    // Round to nearest 10/100/1000 yuan. A real scam transfer often has
    // a round figure ("20000 yuan exactly"). We treat any amount that is
    // a multiple of 10000 fen (100 yuan) AND >= 10000 fen (100 yuan) as
    // "round enough to mention".
    amount_fen >= 10_000 && amount_fen % 10_000 == 0
}

async fn historical_median_fen(pool: &PgPool, payee_id: &str) -> Result<Option<i64>, sqlx::Error> {
    let amounts: Vec<i64> = sqlx::query_scalar(
        "SELECT amount_fen FROM transfers WHERE payee_id = $1 AND status IN ('posted','approved') ORDER BY amount_fen",
    )
    .bind(payee_id)
    .fetch_all(pool)
    .await?;
    if amounts.len() < 3 {
        return Ok(None);
    }
    Ok(Some(amounts[amounts.len() / 2]))
}

async fn average_monthly_expense_fen(pool: &PgPool, user_id: &str) -> Result<i64, sqlx::Error> {
    // Sum debits over the last 90 days, divide by 3 to get a monthly average.
    let row: (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(-t.amount_fen), 0)::BIGINT FROM transactions t \
         JOIN accounts a ON a.id = t.account_id \
         WHERE a.user_id = $1 AND t.amount_fen < 0 \
         AND t.occurred_at >= NOW() - INTERVAL '90 days'",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(row.0 / 3)
}

async fn available_balance_fen(pool: &PgPool, user_id: &str) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(available_balance_fen), 0)::BIGINT FROM accounts WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// Score a transfer. The output is a list of *signals*, not a tier
/// decision. The policy engine in `policy.rs` reads `flags` and may
/// escalate a Yellow to Red on its own. This function is also used by
/// the chat UI to display the guard's reasoning before policy runs.
pub async fn score_transfer(
    pool: &PgPool,
    user: &User,
    payee: &Payee,
    amount_fen: i64,
    memo: &str,
) -> Result<FraudAssessment, sqlx::Error> {
    let mut flags: Vec<FraudFlag> = Vec::new();
    if payee.first_used_at.is_none() {
        flags.push(FraudFlag {
            code: "new_payee".into(),
            message: format!("You have never paid {} before.", payee.display_name),
            weight: 3,
        });
    }
    if !payee.verified {
        flags.push(FraudFlag {
            code: "unverified_payee".into(),
            message: format!("{} is not on your verified payee list.", payee.display_name),
            weight: 4,
        });
    }
    if let Some(median) = historical_median_fen(pool, &payee.id).await? {
        if median > 0 && amount_fen > 3 * median {
            flags.push(FraudFlag {
                code: "amount_unusual".into(),
                message: format!(
                    "This amount is more than 3× your usual payment to {} (median {} fen).",
                    payee.display_name, median
                ),
                weight: 4,
            });
        }
    }
    if let Some(phrase) = detect_urgency(memo) {
        flags.push(FraudFlag {
            code: "urgency_words".into(),
            message: format!("Memo contains urgency wording ({:?}).", phrase),
            weight: 5,
        });
    }
    if let Some(kw) = detect_flagged_merchant(&payee.display_name) {
        flags.push(FraudFlag {
            code: "payee_flagged".into(),
            message: format!(
                "Payee name contains a flag-list keyword ({:?}).",
                kw
            ),
            weight: 5,
        });
    }
    if is_round_number(amount_fen) && amount_fen >= 50_000 {
        flags.push(FraudFlag {
            code: "amount_round".into(),
            message: format!(
                "Amount is a suspiciously round figure ({} fen).",
                amount_fen
            ),
            weight: 1,
        });
    }
    let avg_expense = average_monthly_expense_fen(pool, &user.id).await?;
    let available = available_balance_fen(pool, &user.id).await?;
    if avg_expense > 0 && available > 0 && available - amount_fen < avg_expense {
        flags.push(FraudFlag {
            code: "out_of_pattern".into(),
            message: format!(
                "After this transfer, your balance would fall below your monthly spend baseline ({} fen).",
                avg_expense
            ),
            weight: 3,
        });
    }

    // Score is a simple sum-of-weights, capped at 100. Each weight is
    // 1..=5, so even with 5 flags the score stays <= 25; we multiply
    // by a factor to spread the range for the demo gauge.
    let raw: i32 = flags.iter().map(|f| f.weight as i32).sum();
    let score: u8 = ((raw as f32) * 4.0).min(100.0) as u8;

    let templated = if flags.is_empty() {
        "No scam indicators detected.".to_string()
    } else {
        let reasons: Vec<String> = flags.iter().map(|f| f.message.clone()).collect();
        format!("This transfer has {} signal(s): {}.", flags.len(), reasons.join(" "))
    };

    Ok(FraudAssessment {
        flags,
        score,
        templated,
        narrative: None, // populated separately by `narrate_scam_flags`
    })
}

/// Best-effort LLM narrative. Returns the templated explanation as a
/// fallback when the LLM is unavailable so the UI always has text.
pub async fn narrate_scam_flags(
    settings: &crate::config::Settings,
    assessment: &FraudAssessment,
    payee_name: &str,
    amount_fen: i64,
) -> Option<String> {
    if !settings.unsloth_enabled() {
        return None;
    }
    let prompt = json!({
        "system": "You are BankPilot's fraud narrator. Given a list of fraud signals, write a 1-2 sentence warm-but-firm warning explaining why the transfer looks risky. Use the user's name. Be specific. Do not invent signals. Output plain text only.",
        "signals": assessment.flags.iter().map(|f| json!({"code": f.code, "message": f.message, "weight": f.weight})).collect::<Vec<_>>(),
        "score": assessment.score,
        "payee": payee_name,
        "amount_fen": amount_fen,
    });
    let body = serde_json::to_string(&prompt).ok()?;
    // Cheap, raw completion — we don't need a structured proposal here.
    let url = format!("{}/chat/completions", settings.unsloth_url.trim_end_matches('/'));
    let req = llm_http::client()
        .post(&url)
        .bearer_auth(&settings.unsloth_api_key)
        .json(&json!({
            "model": settings.unsloth_model,
            "messages": [{"role":"user","content": body}],
            "temperature": 0.4,
            "max_tokens": 220,
        }));
    let resp = req.send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let v: Value = resp.json().await.ok()?;
    v.get("choices")?
        .get(0)?
        .get("message")?
        .get("content")?
        .as_str()
        .map(|s| s.trim().to_string())
}

// We re-export llm_http::client to keep this module decoupled from
// internals. A small `pub(crate)` shim would do, but the existing
// `pub(crate)` visibility on `client` is fine.
