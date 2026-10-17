//! Human-readable explanations for policy decisions.
//!
//! The policy engine in `policy.rs` returns a `Decision` with structured
//! fields (tier, reason, flags). The chat UI wants a one-sentence summary
//! the user can read: "This transfer needs your explicit confirmation +
//! mock MFA because it would put you over today's ¥5,000 limit."
//!
//! This module owns that translation. It does NOT take action — only
//! renders text. The route surfaces the explanation in the proposal
//! response so the user understands *why* the plan is Red.

use crate::money::fen_to_cny_str;
use crate::policy::Decision;

pub struct Explanation {
    pub headline: String,
    pub body: String,
    pub tier: String,
}

pub fn explain_transfer(
    d: &Decision,
    amount_fen: i64,
    payee_display: Option<&str>,
) -> Explanation {
    let tier = d.tier.clone();
    let formatted = fen_to_cny_str(amount_fen);
    let headline = match d.tier.as_str() {
        "Red" => "Heads up: this transfer needs extra verification.".to_string(),
        "Yellow" => "Quick confirmation needed.".to_string(),
        "Green" => "No confirmation needed for this one.".to_string(),
        other => format!("{other} tier."),
    };
    let body = match d.reason.as_str() {
        "non_positive_amount" => "The amount has to be positive.".to_string(),
        "injection_pattern" => format!(
            "We blocked this because the message contains a phrase that looks like a prompt-injection attempt (`{}`).",
            d.flags.first().cloned().unwrap_or_default()
        ),
        "daily_limit_exceeded" => format!(
            "This {formatted} transfer would put you over today's spending limit."
        ),
        "amount_unusual_for_payee" | "new_payee" | "unverified_payee" => {
            let who = payee_display.unwrap_or("this payee");
            let mut bits: Vec<String> = Vec::new();
            if d.reason == "amount_unusual_for_payee" || d.flags.iter().any(|f| f == "amount_unusual_for_payee") {
                bits.push(format!("the amount {formatted} is much larger than what you usually send to {who}"));
            }
            if d.flags.iter().any(|f| f == "new_payee") {
                bits.push(format!("{who} is a new payee"));
            }
            if d.flags.iter().any(|f| f == "unverified_payee") {
                bits.push(format!("{who} isn't verified yet"));
            }
            format!("A few things make this unusual: {}.", bits.join("; "))
        }
        "within_daily_limit" => format!(
            "Transfer {formatted} stays within today's limit and matches your usual pattern with this payee."
        ),
        other => format!("Reason: {other}."),
    };
    Explanation { headline, body, tier }
}

pub fn explain_wealth(d: &Decision, product_name: &str, suitability_denied: Option<&str>) -> Explanation {
    let tier = d.tier.clone();
    let headline = match d.tier.as_str() {
        "Red" => "We can't recommend this product for you.".to_string(),
        "Yellow" => "This investment is OK but you need to confirm.".to_string(),
        _ => format!("Product: {product_name}"),
    };
    let body = match (d.reason.as_str(), suitability_denied) {
        (_, Some(why)) => format!("Suitability check blocked this: {why}."),
        ("high_risk_product", _) => format!(
            "{product_name} is a high-risk product. The policy engine won't allow it without explicit approval."
        ),
        ("suitability_pass", _) => format!(
            "You match the suitability profile for {product_name}; you'll still see a confirmation step."
        ),
        (other, _) => format!("Reason: {other}."),
    };
    Explanation { headline, body, tier }
}

pub fn explain_card(kind: &str, d: &Decision) -> Explanation {
    let headline = match kind {
        "issue" => "Issuing a virtual card needs your confirmation.".to_string(),
        "freeze" => "Freezing a card is a one-tap action — confirm to proceed.".to_string(),
        "unfreeze" => "Unfreezing needs your confirmation.".to_string(),
        "cancel" => "Cancelling a card is permanent. Please confirm.".to_string(),
        "report_lost" => "Reporting a card lost triggers immediate block + reissue.".to_string(),
        _ => "Card action.".to_string(),
    };
    Explanation {
        headline,
        body: format!("Reason: {}.", d.reason),
        tier: d.tier.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explain_daily_limit_uses_amount() {
        let d = Decision {
            tier: "Red".into(),
            reason: "daily_limit_exceeded".into(),
            escalate_from: Some("Yellow".into()),
            flags: vec!["today_fen=480000".into(), "attempted_fen=200000".into()],
        };
        let e = explain_transfer(&d, 200000, Some("Li Wei"));
        assert!(e.body.contains("limit"));
        assert!(e.body.contains("200"));
    }

    #[test]
    fn explain_unverified_payee() {
        let d = Decision {
            tier: "Red".into(),
            reason: "unverified_payee".into(),
            escalate_from: Some("Yellow".into()),
            flags: vec!["unverified_payee".into(), "new_payee".into()],
        };
        let e = explain_transfer(&d, 20000, Some("Cake Shop"));
        assert!(e.body.contains("Cake Shop"));
    }
}
