//! Policy engine — the only place risk tier is decided.
//!
//! Tiers: Green (read-only) / Yellow (single confirm) / Red (confirm + mock MFA).

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;

use crate::config;
use crate::errors;
use crate::models::{Account, Payee, User, WealthProduct};

pub const GREEN: &str = "Green";
pub const YELLOW: &str = "Yellow";
pub const RED: &str = "Red";

#[derive(Debug, Clone, Serialize)]
pub struct Decision {
    pub tier: String,
    pub reason: String,
    pub escalate_from: Option<String>,
    pub flags: Vec<String>,
}

const INJECTION_PHRASES: &[&str] = &[
    "ignore previous",
    "ignore all",
    "ignore above",
    "disregard previous",
    "transfer all",
    "transfer everything",
    "all funds",
    "override policy",
    "you are now",
    "system message",
    "new instructions",
];

pub fn detect_injection(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    for phrase in INJECTION_PHRASES {
        if lower.contains(phrase) {
            return Some(phrase.to_string());
        }
    }
    None
}

pub async fn today_total_fen(pool: &PgPool, account_id: &str) -> Result<i64, sqlx::Error> {
    let now: DateTime<Utc> = Utc::now();
    let start = now
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();

    let rows: Vec<(String, Option<DateTime<Utc>>, i64)> = sqlx::query_as(
        "SELECT status, posted_at, amount_fen FROM transfers WHERE source_account_id = $1",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?;
    let mut total = 0i64;
    for (status, posted_at, amount) in rows {
        if matches!(status.as_str(), "posted" | "approved") {
            if let Some(p) = posted_at {
                if p >= start {
                    total += amount;
                }
            }
        }
    }
    Ok(total)
}

async fn check_daily_limit(
    pool: &PgPool,
    account_id: &str,
    amount_fen: i64,
) -> Result<Option<Decision>, sqlx::Error> {
    let settings = config::get_settings();
    let today = today_total_fen(pool, account_id).await?;
    if today + amount_fen > settings.daily_limit_fen {
        Ok(Some(Decision {
            tier: RED.to_string(),
            reason: "daily_limit_exceeded".to_string(),
            escalate_from: Some(YELLOW.to_string()),
            flags: vec![format!("today_fen={today}"), format!("attempted_fen={amount_fen}")],
        }))
    } else {
        Ok(None)
    }
}

fn check_payee_escalation(payee: &Payee) -> Vec<String> {
    let mut flags = vec![];
    if !payee.verified {
        flags.push("unverified_payee".to_string());
    }
    if payee.first_used_at.is_none() {
        flags.push("new_payee".to_string());
    }
    flags
}

async fn check_amount_unusual(
    pool: &PgPool,
    payee: &Payee,
    amount_fen: i64,
) -> Result<Vec<String>, sqlx::Error> {
    let amounts: Vec<i64> =
        sqlx::query_scalar("SELECT amount_fen FROM transfers WHERE payee_id = $1 AND status = 'posted' ORDER BY amount_fen")
            .bind(&payee.id)
            .fetch_all(pool)
            .await?;
    if amounts.len() < 3 {
        return Ok(vec![]);
    }
    let median = amounts[amounts.len() / 2];
    if median > 0 && amount_fen > 3 * median {
        Ok(vec!["amount_unusual_for_payee".to_string()])
    } else {
        Ok(vec![])
    }
}

pub enum DecideResult {
    Decision(Decision),
    SuitabilityDenied(String),
}

pub async fn decide_transfer(
    pool: &PgPool,
    user: &User,
    account: &Account,
    payee: &Payee,
    amount_fen: i64,
    memo: &str,
) -> Result<Decision, sqlx::Error> {
    if amount_fen <= 0 {
        return Ok(Decision {
            tier: RED.to_string(),
            reason: "non_positive_amount".to_string(),
            escalate_from: None,
            flags: vec![],
        });
    }
    if let Some(phrase) = detect_injection(memo) {
        return Ok(Decision {
            tier: RED.to_string(),
            reason: "injection_pattern".to_string(),
            escalate_from: None,
            flags: vec![format!("injection:{phrase}")],
        });
    }
    let mut flags: Vec<String> = check_payee_escalation(payee);
    flags.extend(check_amount_unusual(pool, payee, amount_fen).await?);
    if let Some(limit_decision) = check_daily_limit(pool, &account.id, amount_fen).await? {
        flags.extend(limit_decision.flags);
        return Ok(Decision {
            tier: RED.to_string(),
            reason: limit_decision.reason,
            escalate_from: Some(YELLOW.to_string()),
            flags,
        });
    }
    // Scam/fraud guard: pure-Rust rules that run *after* the policy's
    // own checks. A flag with a hard-signal code (urgency_words,
    // payee_flagged, amount_unusual with weight >= 4) escalates the
    // tier to Red. We also accumulate the flag list so the chat UI can
    // surface the explanation alongside the tier.
    let assessment = crate::fraud::score_transfer(pool, user, payee, amount_fen, memo).await?;
    let mut fraud_flag_strs: Vec<String> = assessment
        .flags
        .iter()
        .map(|f| format!("fraud:{}", f.code))
        .collect();
    let hard = assessment
        .flags
        .iter()
        .any(|f| matches!(f.code.as_str(), "urgency_words" | "payee_flagged") || f.weight >= 5);
    flags.append(&mut fraud_flag_strs);
    if hard {
        return Ok(Decision {
            tier: RED.to_string(),
            reason: "scam_guard_signal".to_string(),
            escalate_from: Some(YELLOW.to_string()),
            flags,
        });
    }
    if !flags.is_empty() {
        return Ok(Decision {
            tier: RED.to_string(),
            reason: flags.join(","),
            escalate_from: Some(YELLOW.to_string()),
            flags,
        });
    }
    Ok(Decision {
        tier: YELLOW.to_string(),
        reason: "within_daily_limit".to_string(),
        escalate_from: None,
        flags: vec![],
    })
}

pub fn decide_card_action(kind: &str) -> Decision {
    match kind {
        "issue" => Decision {
            tier: YELLOW.to_string(),
            reason: "card_issue".to_string(),
            escalate_from: None,
            flags: vec![],
        },
        "freeze" => Decision {
            tier: YELLOW.to_string(),
            reason: "card_freeze".to_string(),
            escalate_from: None,
            flags: vec![],
        },
        "unfreeze" => Decision {
            tier: YELLOW.to_string(),
            reason: "card_unfreeze".to_string(),
            escalate_from: None,
            flags: vec![],
        },
        "cancel" => Decision {
            tier: YELLOW.to_string(),
            reason: "card_cancel".to_string(),
            escalate_from: None,
            flags: vec![],
        },
        "update_limits" => Decision {
            tier: YELLOW.to_string(),
            reason: "card_limit_change".to_string(),
            escalate_from: None,
            flags: vec![],
        },
        "report_lost" => Decision {
            tier: RED.to_string(),
            reason: "card_report_lost".to_string(),
            escalate_from: None,
            flags: vec![],
        },
        _ => Decision {
            tier: RED.to_string(),
            reason: "unknown_card_action".to_string(),
            escalate_from: None,
            flags: vec![],
        },
    }
}

pub fn decide_wealth_subscribe(product: &WealthProduct, user: &User) -> DecideResult {
    if product.risk_level == "high" {
        return DecideResult::Decision(Decision {
            tier: RED.to_string(),
            reason: "high_risk_product".to_string(),
            escalate_from: Some(YELLOW.to_string()),
            flags: vec![],
        });
    }
    let needed = product.suitability_required.trim();
    if needed.starts_with("risk_tolerance:low") && matches!(user.risk_tolerance.as_str(), "medium" | "high") {
        return DecideResult::SuitabilityDenied("low_risk_product_incompatible_with_profile".to_string());
    }
    DecideResult::Decision(Decision {
        tier: YELLOW.to_string(),
        reason: "suitability_pass".to_string(),
        escalate_from: None,
        flags: vec![],
    })
}

pub fn decide_wealth_redeem() -> Decision {
    Decision {
        tier: YELLOW.to_string(),
        reason: "wealth_redeem".to_string(),
        escalate_from: None,
        flags: vec![],
    }
}

pub fn decide_subscription_cancel() -> Decision {
    Decision {
        tier: YELLOW.to_string(),
        reason: "subscription_cancel".to_string(),
        escalate_from: None,
        flags: vec![],
    }
}

pub fn decide_goal_reservation() -> Decision {
    Decision {
        tier: YELLOW.to_string(),
        reason: "goal_reservation".to_string(),
        escalate_from: None,
        flags: vec![],
    }
}

/// Set / update a budget cap. Same tier as a goal reservation — the user is
/// promising to constrain future spend, not moving money.
pub fn decide_budget_set() -> Decision {
    Decision {
        tier: YELLOW.to_string(),
        reason: "budget_set".to_string(),
        escalate_from: None,
        flags: vec![],
    }
}

/// Reserve cash against a future event (e.g. spouse's birthday). Moves
/// money from available → reserved, so the policy treats it as Red — same
/// tier as a transfer of the same size.
pub fn decide_budget_reserve() -> Decision {
    Decision {
        tier: RED.to_string(),
        reason: "budget_reserve".to_string(),
        escalate_from: None,
        flags: vec![],
    }
}

pub fn assert_unlocked(user: &User) -> Result<(), errors::ApiError> {
    if user.safety_lock_state == "locked" {
        return Err(errors::safety_locked(user.safety_lock_reason.as_deref()));
    }
    Ok(())
}
