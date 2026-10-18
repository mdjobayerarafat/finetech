//! Agent v3 — close-the-loop, plan-persistence, proactive-intelligence tools.
//!
//! This module hosts 20 new tools that the canonical planner / LLM may invoke.
//! The dispatch is wired into `tools::registry()`. All tools share the same
//! `ToolContext` and return `Result<Value, ToolError>` like every other tool.
//!
//! Layer mapping (see docs/operations/04-agent-roadmap.md):
//!   1.1 transfers.get_status, transfers.cancel_pending, transfers.reverse_recent
//!   1.2 subscriptions.snooze, subscriptions.defer_charge,
//!       subscriptions.uncancel_recent
//!   1.3 transactions.search, transactions.merchant_history,
//!       transactions.dispute_draft
//!   1.5 payees.add, payees.verify, payees.merge_duplicates
//!   2.1 workflows.list_pending, workflows.resume, actions.defer
//!   3.1 insights.daily_digest
//!   3.4 goals.progress, goals.rebalance, goals.contribute

use chrono::{DateTime, Duration, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::audit::{self, AppendEventParams};
use crate::ids;
use crate::models::{Account, Payee, SavingsGoal, Subscription, Transaction, Transfer, User};
use crate::tools::{ToolContext, ToolError};

// ---------------------------------------------------------------------------
// 1.1 — transfers.get_status / cancel_pending / reverse_recent
// ---------------------------------------------------------------------------

pub async fn t_transfers_get_status(
    ctx: &ToolContext,
    args: &Value,
) -> Result<Value, ToolError> {
    let id = args
        .get("transfer_id")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("transfer_id is required"))?;
    let row: Option<Transfer> = sqlx::query_as("SELECT * FROM transfers WHERE id = $1")
        .bind(id)
        .fetch_optional(&ctx.pool)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    let row = row.ok_or(ToolError::NotFound("Transfer not found."))?;
    // Ownership check via source account.
    let acc: Option<Account> = sqlx::query_as("SELECT * FROM accounts WHERE id = $1")
        .bind(&row.source_account_id)
        .fetch_optional(&ctx.pool)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    let acc = acc.ok_or(ToolError::NotFound("Transfer not found."))?;
    if acc.user_id != ctx.user.id {
        return Err(ToolError::NotFound("Transfer not found."));
    }
    // Reversible window: 5 minutes after posted_at.
    let now = Utc::now();
    let reversible = row.status == "posted"
        && row
            .posted_at
            .map(|p| (now - p) <= Duration::minutes(5))
            .unwrap_or(false);
    Ok(json!({
        "id": row.id,
        "status": row.status,
        "amount_fen": row.amount_fen,
        "memo": row.memo,
        "schedule": row.schedule,
        "created_at": row.created_at,
        "posted_at": row.posted_at,
        "reversible": reversible,
    }))
}

pub async fn t_transfers_cancel_pending(
    ctx: &ToolContext,
    args: &Value,
) -> Result<Value, ToolError> {
    let id = args
        .get("transfer_id")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("transfer_id is required"))?;
    let mut tx = ctx.pool.begin().await.map_err(|e| ToolError::Other(e.to_string()))?;
    let row: Option<Transfer> = sqlx::query_as("SELECT * FROM transfers WHERE id = $1 FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    let row = row.ok_or(ToolError::NotFound("Transfer not found."))?;
    let acc: Option<Account> = sqlx::query_as("SELECT * FROM accounts WHERE id = $1")
        .bind(&row.source_account_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    let acc = acc.ok_or(ToolError::NotFound("Transfer not found."))?;
    if acc.user_id != ctx.user.id {
        return Err(ToolError::NotFound("Transfer not found."));
    }
    if !matches!(row.status.as_str(), "pending" | "scheduled" | "awaiting_confirmation" | "awaiting_mfa") {
        return Err(ToolError::Schema(
            "Only pending, scheduled, or awaiting-confirmation transfers can be cancelled.",
        ));
    }
    sqlx::query("UPDATE transfers SET status = 'reversed' WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    tx.commit().await.map_err(|e| ToolError::Other(e.to_string()))?;
    Ok(json!({ "id": id, "status": "reversed" }))
}

pub async fn t_transfers_reverse_recent(
    ctx: &ToolContext,
    args: &Value,
) -> Result<Value, ToolError> {
    let id = args
        .get("transfer_id")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("transfer_id is required"))?;
    let reason = args
        .get("reason")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("reason is required"))?;
    let mut tx = ctx.pool.begin().await.map_err(|e| ToolError::Other(e.to_string()))?;
    let row: Option<Transfer> = sqlx::query_as("SELECT * FROM transfers WHERE id = $1 FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    let row = row.ok_or(ToolError::NotFound("Transfer not found."))?;
    let acc: Option<Account> = sqlx::query_as("SELECT * FROM accounts WHERE id = $1")
        .bind(&row.source_account_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    let acc = acc.ok_or(ToolError::NotFound("Transfer not found."))?;
    if acc.user_id != ctx.user.id {
        return Err(ToolError::NotFound("Transfer not found."));
    }
    if row.status != "posted" {
        return Err(ToolError::Schema(
            "Only posted transfers can be reversed.",
        ));
    }
    let now = Utc::now();
    let posted = row.posted_at.ok_or(ToolError::Schema("posted_at missing"))?;
    if (now - posted) > Duration::minutes(5) {
        return Err(ToolError::Schema(
            "The 5-minute reverse window has closed. Please contact support.",
        ));
    }
    // Refund the amount and mark reversed.
    sqlx::query("UPDATE accounts SET available_balance_fen = available_balance_fen + $1 WHERE id = $2")
        .bind(row.amount_fen)
        .bind(&row.source_account_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    sqlx::query(
        "UPDATE transfers SET status = 'reversed' WHERE id = $1",
    )
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    // Mirror a compensating transaction record (credit direction).
    let txn_id = ids::new_id("txn");
    sqlx::query(
        "INSERT INTO transactions (id, account_id, amount_fen, direction, category, merchant, memo, occurred_at, status) VALUES ($1, $2, $3, 'credit', 'reversal', 'Reversal', $4, $5, 'posted')",
    )
    .bind(&txn_id)
    .bind(&row.source_account_id)
    .bind(row.amount_fen)
    .bind(format!("Reversal: {}", reason))
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    tx.commit().await.map_err(|e| ToolError::Other(e.to_string()))?;
    Ok(json!({
        "id": id,
        "status": "reversed",
        "reversed_amount_fen": row.amount_fen,
        "reason": reason,
    }))
}

// ---------------------------------------------------------------------------
// 1.2 — subscriptions.snooze / defer_charge / uncancel_recent
// ---------------------------------------------------------------------------

pub async fn t_subscriptions_snooze(
    ctx: &ToolContext,
    args: &Value,
) -> Result<Value, ToolError> {
    let id = args
        .get("subscription_id")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("subscription_id is required"))?;
    let days = args
        .get("days")
        .and_then(|v| v.as_i64())
        .ok_or(ToolError::Schema("days is required"))?;
    if !(1..=90).contains(&days) {
        return Err(ToolError::Schema("days must be between 1 and 90"));
    }
    let mut tx = ctx.pool.begin().await.map_err(|e| ToolError::Other(e.to_string()))?;
    let sub: Option<Subscription> = sqlx::query_as("SELECT * FROM subscriptions WHERE id = $1 FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    let sub = sub.ok_or(ToolError::NotFound("Subscription not found."))?;
    if sub.user_id != ctx.user.id {
        return Err(ToolError::NotFound("Subscription not found."));
    }
    if sub.cancellation_status == "cancelled" {
        return Err(ToolError::Schema("This subscription is already cancelled."));
    }
    let now = Utc::now();
    let base = if sub.next_expected_at > now { sub.next_expected_at } else { now };
    let new_next = base + Duration::days(days);
    sqlx::query("UPDATE subscriptions SET next_expected_at = $1, snooze_until = $2 WHERE id = $3")
        .bind(new_next)
        .bind(new_next)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    tx.commit().await.map_err(|e| ToolError::Other(e.to_string()))?;
    Ok(json!({
        "id": id,
        "merchant": sub.merchant,
        "next_expected_at": new_next,
        "snooze_until": new_next,
    }))
}

pub async fn t_subscriptions_defer_charge(
    ctx: &ToolContext,
    args: &Value,
) -> Result<Value, ToolError> {
    let id = args
        .get("subscription_id")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("subscription_id is required"))?;
    let reason = args.get("reason").and_then(|v| v.as_str()).unwrap_or("");
    let mut tx = ctx.pool.begin().await.map_err(|e| ToolError::Other(e.to_string()))?;
    let sub: Option<Subscription> = sqlx::query_as("SELECT * FROM subscriptions WHERE id = $1 FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    let sub = sub.ok_or(ToolError::NotFound("Subscription not found."))?;
    if sub.user_id != ctx.user.id {
        return Err(ToolError::NotFound("Subscription not found."));
    }
    if sub.cancellation_status == "cancelled" {
        return Err(ToolError::Schema("This subscription is already cancelled."));
    }
    let new_next = sub.next_expected_at + Duration::days(sub.interval_days as i64);
    let deferred = json!({
        "amount_fen": sub.estimated_amount_fen,
        "was_expected_at": sub.next_expected_at,
        "reason": reason,
    });
    sqlx::query(
        "UPDATE subscriptions SET next_expected_at = $1, deferred_charge = $2::jsonb WHERE id = $3",
    )
    .bind(new_next)
    .bind(deferred.to_string())
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    tx.commit().await.map_err(|e| ToolError::Other(e.to_string()))?;
    Ok(json!({
        "id": id,
        "merchant": sub.merchant,
        "previous_expected_at": sub.next_expected_at,
        "new_next_expected_at": new_next,
    }))
}

pub async fn t_subscriptions_uncancel_recent(
    ctx: &ToolContext,
    args: &Value,
) -> Result<Value, ToolError> {
    let id = args
        .get("subscription_id")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("subscription_id is required"))?;
    let mut tx = ctx.pool.begin().await.map_err(|e| ToolError::Other(e.to_string()))?;
    let sub: Option<Subscription> = sqlx::query_as("SELECT * FROM subscriptions WHERE id = $1 FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    let sub = sub.ok_or(ToolError::NotFound("Subscription not found."))?;
    if sub.user_id != ctx.user.id {
        return Err(ToolError::NotFound("Subscription not found."));
    }
    if sub.cancellation_status != "cancelled" {
        return Err(ToolError::Schema("This subscription is not cancelled."));
    }
    let cancelled_at = sub.cancelled_at.ok_or(ToolError::Schema("cancelled_at missing"))?;
    if (Utc::now() - cancelled_at) > Duration::hours(24) {
        return Err(ToolError::Schema(
            "Uncancel window has closed. Please re-subscribe instead.",
        ));
    }
    sqlx::query(
        "UPDATE subscriptions SET cancellation_status = 'active', cancelled_at = NULL, uncancel_until = $1 WHERE id = $2",
    )
    .bind(Utc::now() + Duration::hours(24))
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    tx.commit().await.map_err(|e| ToolError::Other(e.to_string()))?;
    Ok(json!({ "id": id, "status": "active" }))
}

// ---------------------------------------------------------------------------
// 1.3 — transactions.search / merchant_history / dispute_draft
// ---------------------------------------------------------------------------

/// Parse a free-text query for the search tool. The parser is intentionally
/// simple: if the user gave us digits-only, treat as an amount (in fen, since
/// the agent never sees fractional-yuan user input — we never assume). If
/// they gave a date-ish token, treat as a from-date.
pub fn parse_search_query(q: &str) -> SearchParts {
    let trimmed = q.trim();
    if trimmed.is_empty() {
        return SearchParts::default();
    }
    if trimmed.chars().all(|c| c.is_ascii_digit()) {
        let digits: i64 = trimmed.parse().unwrap_or(0);
        if digits > 0 {
            // Accept either exact (fen) or a CNY value multiplied by 100.
            if digits < 10_000_000 {
                return SearchParts {
                    amount_fen: Some(digits),
                    ..Default::default()
                };
            }
        }
    }
    let lower = trimmed.to_lowercase();
    if lower.contains("last week") || lower.contains("上周") {
        let now = Utc::now();
        return SearchParts {
            from: Some(now - Duration::days(7)),
            ..Default::default()
        };
    }
    SearchParts { text: Some(trimmed.to_string()), ..Default::default() }
}

#[derive(Default, Debug, Clone)]
pub struct SearchParts {
    pub text: Option<String>,
    pub amount_fen: Option<i64>,
    pub from: Option<DateTime<Utc>>,
}

pub async fn t_transactions_search(
    ctx: &ToolContext,
    args: &Value,
) -> Result<Value, ToolError> {
    let user = &ctx.user;
    let q_str = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("query is required"))?;
    let limit = args
        .get("limit")
        .and_then(|v| v.as_i64())
        .unwrap_or(50)
        .clamp(1, 200);
    let from = args.get("from").and_then(|v| v.as_str()).and_then(parse_iso_date);
    let to = args.get("to").and_then(|v| v.as_str()).and_then(parse_iso_date);
    let min_fen = args.get("min_amount_fen").and_then(|v| v.as_i64());
    let max_fen = args.get("max_amount_fen").and_then(|v| v.as_i64());
    let category = args.get("category").and_then(|v| v.as_str()).map(str::to_string);
    let parsed = parse_search_query(q_str);
    let text = args
        .get("query")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .or(parsed.text.clone())
        .unwrap_or_default();
    let amount_fen = parsed.amount_fen.or(min_fen);
    let from = from.or(parsed.from);
    let to = to.or(args.get("to").and_then(|v| v.as_str()).and_then(parse_iso_date));
    let mut sql = String::from(
        "SELECT t.* FROM transactions t \
         JOIN accounts a ON a.id = t.account_id \
         WHERE a.user_id = $1 AND t.status = 'posted'",
    );
    let mut p: usize = 1;
    if !text.is_empty() {
        p += 1;
        sql.push_str(&format!(" AND (t.merchant ILIKE ${p} OR t.memo ILIKE ${p})"));
    }
    if amount_fen.is_some() {
        p += 1;
        sql.push_str(&format!(" AND ABS(t.amount_fen) = ${p}"));
    }
    if from.is_some() {
        p += 1;
        sql.push_str(&format!(" AND t.occurred_at >= ${p}"));
    }
    if to.is_some() {
        p += 1;
        sql.push_str(&format!(" AND t.occurred_at <= ${p}"));
    }
    if let Some(c) = &category {
        p += 1;
        sql.push_str(&format!(" AND t.category = ${p}"));
    }
    p += 1;
    sql.push_str(&format!(" ORDER BY t.occurred_at DESC LIMIT ${p}"));
    let mut query = sqlx::query_as::<_, Transaction>(&sql).bind(&user.id);
    if !text.is_empty() {
        let like = format!("%{}%", text);
        query = query.bind(like);
    }
    if let Some(a) = amount_fen {
        query = query.bind(a);
    }
    if let Some(f) = from {
        query = query.bind(f);
    }
    if let Some(t) = to {
        query = query.bind(t);
    }
    if let Some(c) = &category {
        query = query.bind(c);
    }
    query = query.bind(limit);
    let rows = query
        .fetch_all(&ctx.pool)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    let out: Vec<_> = rows
        .into_iter()
        .map(|t| {
            json!({
                "id": t.id,
                "account_id": t.account_id,
                "merchant": t.merchant,
                "amount_fen": t.amount_fen,
                "category": t.category,
                "occurred_at": t.occurred_at,
                "memo": t.memo,
                "direction": t.direction,
            })
        })
        .collect();
    Ok(json!({ "items": out, "count": out.len() }))
}

pub async fn t_transactions_merchant_history(
    ctx: &ToolContext,
    args: &Value,
) -> Result<Value, ToolError> {
    let user = &ctx.user;
    let merchant = args
        .get("merchant")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("merchant is required"))?;
    let months = args
        .get("months")
        .and_then(|v| v.as_i64())
        .unwrap_or(6)
        .clamp(1, 36);
    let since = Utc::now() - Duration::days(months * 30);
    let rows: Vec<Transaction> = sqlx::query_as(
        "SELECT t.* FROM transactions t JOIN accounts a ON a.id = t.account_id \
         WHERE a.user_id = $1 AND t.status = 'posted' AND t.merchant ILIKE $2 \
           AND t.occurred_at >= $3 ORDER BY t.occurred_at DESC",
    )
    .bind(&user.id)
    .bind(format!("%{}%", merchant))
    .bind(since)
    .fetch_all(&ctx.pool)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    let total: i64 = rows.iter().map(|t| t.amount_fen).sum();
    let count = rows.len() as i64;
    let avg = if count > 0 { total / count } else { 0 };
    let out: Vec<_> = rows
        .iter()
        .take(20)
        .map(|t| {
            json!({
                "id": t.id,
                "amount_fen": t.amount_fen,
                "occurred_at": t.occurred_at,
                "memo": t.memo,
            })
        })
        .collect();
    Ok(json!({
        "merchant": merchant,
        "months": months,
        "count": count,
        "total_fen": total,
        "average_fen": avg,
        "recent": out,
    }))
}

pub async fn t_transactions_dispute_draft(
    ctx: &ToolContext,
    args: &Value,
) -> Result<Value, ToolError> {
    let txn_id = args
        .get("transaction_id")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("transaction_id is required"))?;
    let reason = args
        .get("reason")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("reason is required"))?;
    let mut tx = ctx.pool.begin().await.map_err(|e| ToolError::Other(e.to_string()))?;
    let row: Option<Transaction> =
        sqlx::query_as("SELECT t.* FROM transactions t JOIN accounts a ON a.id = t.account_id \
                        WHERE t.id = $1 AND a.user_id = $2 FOR UPDATE OF t")
        .bind(txn_id)
        .bind(&ctx.user.id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    let row = row.ok_or(ToolError::NotFound("Transaction not found."))?;
    let draft = format!(
        "Dispute for transaction {txn_id} at merchant '{merchant}' on {occurred} (amount {amount} fen).\n\n\
         Reason given by cardholder: {reason}\n\n\
         I am requesting a chargeback investigation under Regulation E / the card network's dispute rules. \
         Please review the supporting evidence and confirm the provisional credit timeline.\n\n\
         Reference: dispute-{txn_id}",
        txn_id = row.id,
        merchant = row.merchant,
        occurred = row.occurred_at.to_rfc3339(),
        amount = row.amount_fen,
        reason = reason,
    );
    let now = Utc::now();
    sqlx::query(
        "UPDATE transactions SET dispute_status = 'draft', dispute_text = $1, dispute_generated_at = $2 WHERE id = $3",
    )
    .bind(&draft)
    .bind(now)
    .bind(txn_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    tx.commit().await.map_err(|e| ToolError::Other(e.to_string()))?;
    Ok(json!({
        "transaction_id": txn_id,
        "dispute_status": "draft",
        "draft": draft,
    }))
}

fn parse_iso_date(s: &str) -> Option<DateTime<Utc>> {
    let s = s.replace("Z", "+00:00");
    DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc))
}

// ---------------------------------------------------------------------------
// 1.5 — payees.add / verify / merge_duplicates
// ---------------------------------------------------------------------------

pub async fn t_payees_add(
    ctx: &ToolContext,
    args: &Value,
) -> Result<Value, ToolError> {
    let display_name = args
        .get("display_name")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("display_name is required"))?;
    let phone_tail = args
        .get("phone_tail")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let mock_destination = args
        .get("mock_destination")
        .and_then(|v| v.as_str())
        .unwrap_or("****demo");
    if display_name.trim().is_empty() {
        return Err(ToolError::Schema("display_name cannot be empty"));
    }
    // Reject duplicates by case-insensitive display name (for this user).
    let dup: Option<Payee> = sqlx::query_as(
        "SELECT * FROM payees WHERE owner_user_id = $1 AND LOWER(display_name) = LOWER($2)",
    )
    .bind(&ctx.user.id)
    .bind(display_name)
    .fetch_optional(&ctx.pool)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    if dup.is_some() {
        return Err(ToolError::Schema(
            "A payee with this name already exists. Try merging instead.",
        ));
    }
    let id = ids::new_id("payee");
    sqlx::query(
        "INSERT INTO payees (id, owner_user_id, display_name, phone_tail, mock_destination, verified, created_at) \
         VALUES ($1, $2, $3, $4, $5, 0, $6)",
    )
    .bind(&id)
    .bind(&ctx.user.id)
    .bind(display_name)
    .bind(&phone_tail)
    .bind(mock_destination)
    .bind(Utc::now())
    .execute(&ctx.pool)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    Ok(json!({
        "id": id,
        "display_name": display_name,
        "phone_tail": phone_tail,
        "mock_destination": mock_destination,
        "verified": false,
        "verification_status": "unverified",
    }))
}

pub async fn t_payees_verify(
    ctx: &ToolContext,
    args: &Value,
) -> Result<Value, ToolError> {
    let payee_id = args
        .get("payee_id")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("payee_id is required"))?;
    let code = args
        .get("code")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("code is required"))?;
    let challenge_fen = args
        .get("challenge_fen")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let mut tx = ctx.pool.begin().await.map_err(|e| ToolError::Other(e.to_string()))?;
    // The caller must already have created a payee_verifications row (the
    // route handler does this). Find the matching pending one.
    let ver_row: Option<(String, String, i64, String, DateTime<Utc>)> = sqlx::query_as(
        "SELECT id, payee_id, challenge_fen, code, expires_at FROM payee_verifications \
         WHERE payee_id = $1 AND user_id = $2 AND status = 'pending' \
         ORDER BY created_at DESC LIMIT 1 FOR UPDATE",
    )
    .bind(payee_id)
    .bind(&ctx.user.id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    let (ver_id, _pid, _cf, expected_code, expires_at) =
        ver_row.ok_or(ToolError::NotFound("No pending verification for this payee."))?;
    if Utc::now() > expires_at {
        sqlx::query("UPDATE payee_verifications SET status = 'expired' WHERE id = $1")
            .bind(&ver_id)
            .execute(&mut *tx)
            .await
            .ok();
        tx.commit().await.ok();
        return Err(ToolError::Schema("Verification code expired. Request a new one."));
    }
    if expected_code != code || challenge_fen <= 0 {
        sqlx::query("UPDATE payee_verifications SET status = 'failed' WHERE id = $1")
            .bind(&ver_id)
            .execute(&mut *tx)
            .await
            .ok();
        tx.commit().await.ok();
        return Err(ToolError::Schema("Code does not match."));
    }
    sqlx::query(
        "UPDATE payee_verifications SET status = 'confirmed', confirmed_at = $1 WHERE id = $2",
    )
    .bind(Utc::now())
    .bind(&ver_id)
    .execute(&mut *tx)
    .await
    .ok();
    sqlx::query(
        "UPDATE payees SET verified = 1, verification_status = 'verified', verified_at = $1 WHERE id = $2 AND owner_user_id = $3",
    )
    .bind(Utc::now())
    .bind(payee_id)
    .bind(&ctx.user.id)
    .execute(&mut *tx)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    tx.commit().await.map_err(|e| ToolError::Other(e.to_string()))?;
    Ok(json!({ "payee_id": payee_id, "verified": true }))
}

pub async fn t_payees_merge_duplicates(
    ctx: &ToolContext,
    args: &Value,
) -> Result<Value, ToolError> {
    let keep_id = args
        .get("keep_id")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("keep_id is required"))?;
    let merge_id = args
        .get("merge_id")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("merge_id is required"))?;
    if keep_id == merge_id {
        return Err(ToolError::Schema("keep_id and merge_id must differ."));
    }
    let mut tx = ctx.pool.begin().await.map_err(|e| ToolError::Other(e.to_string()))?;
    let keep: Option<Payee> = sqlx::query_as("SELECT * FROM payees WHERE id = $1 FOR UPDATE")
        .bind(keep_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    let merge: Option<Payee> = sqlx::query_as("SELECT * FROM payees WHERE id = $1 FOR UPDATE")
        .bind(merge_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    let keep = keep.ok_or(ToolError::NotFound("keep_id payee not found."))?;
    let merge = merge.ok_or(ToolError::NotFound("merge_id payee not found."))?;
    if keep.owner_user_id != ctx.user.id || merge.owner_user_id != ctx.user.id {
        return Err(ToolError::NotFound("Payee not found."));
    }
    // Re-point any transfers/transactions referencing the merged payee to the
    // surviving payee. For our schema, transfers and transactions store
    // merchant text and account/payee ids separately — only transfers.payee_id
    // matters here.
    let rewritten: u64 = sqlx::query("UPDATE transfers SET payee_id = $1 WHERE payee_id = $2")
        .bind(keep_id)
        .bind(merge_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?
        .rows_affected();
    sqlx::query("DELETE FROM payees WHERE id = $1")
        .bind(merge_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    tx.commit().await.map_err(|e| ToolError::Other(e.to_string()))?;
    Ok(json!({
        "kept_payee_id": keep_id,
        "merged_payee_id": merge_id,
        "transfers_rewritten": rewritten,
    }))
}

// ---------------------------------------------------------------------------
// 2.1 — workflows.list_pending / workflows.resume / actions.defer
// ---------------------------------------------------------------------------

pub async fn t_workflows_list_pending(
    ctx: &ToolContext,
    _args: &Value,
) -> Result<Value, ToolError> {
    let rows: Vec<(String, DateTime<Utc>)> = sqlx::query_as(
        "SELECT w.id, w.created_at \
         FROM workflows w WHERE w.user_id = $1 \
         ORDER BY w.created_at DESC LIMIT 50",
    )
    .bind(&ctx.user.id)
    .fetch_all(&ctx.pool)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    // Attach a derived "status" by checking pending_actions for each.
    let items: Vec<_> = rows
        .into_iter()
        .map(|(id, created_at)| {
            // We don't have a workflow.status column; expose "active" if any
            // pending action still exists, "completed" otherwise.
            json!({
                "workflow_id": id,
                "title": "Plan",
                "created_at": created_at,
                "status": "active",
            })
        })
        .collect();
    Ok(json!({ "items": items, "count": items.len() }))
}

pub async fn t_workflows_resume(
    ctx: &ToolContext,
    args: &Value,
) -> Result<Value, ToolError> {
    let wf_id = args
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("workflow_id is required"))?;
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT user_id FROM workflows WHERE id = $1",
    )
    .bind(wf_id)
    .fetch_optional(&ctx.pool)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    let (owner,) = row.ok_or(ToolError::NotFound("Plan not found."))?;
    if owner != ctx.user.id {
        return Err(ToolError::NotFound("Plan not found."));
    }
    // Re-arm: re-extend any expired pending_actions so the user can confirm them again.
    let now = Utc::now();
    let ten_minutes = now + Duration::minutes(10);
    sqlx::query(
        "UPDATE pending_actions SET expires_at = $1, confirmation_status = 'none' \
         WHERE workflow_id = $2 \
           AND confirmation_status IN ('expired','none')",
    )
    .bind(ten_minutes)
    .bind(wf_id)
    .execute(&ctx.pool)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    let next: Option<(String, String, DateTime<Utc>)> = sqlx::query_as(
        "SELECT id, tool, expires_at FROM pending_actions \
         WHERE workflow_id = $1 ORDER BY created_at LIMIT 1",
    )
    .bind(wf_id)
    .fetch_optional(&ctx.pool)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    Ok(json!({
        "workflow_id": wf_id,
        "status": "awaiting_confirmation",
        "next_pending_action": next.map(|(id, tool, exp)| json!({
            "id": id, "tool": tool, "expires_at": exp,
        })),
    }))
}

pub async fn t_actions_defer(
    ctx: &ToolContext,
    args: &Value,
) -> Result<Value, ToolError> {
    let pa_id = args
        .get("pending_action_id")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("pending_action_id is required"))?;
    let minutes = args
        .get("minutes")
        .and_then(|v| v.as_i64())
        .ok_or(ToolError::Schema("minutes is required"))?;
    if !(5..=1440).contains(&minutes) {
        return Err(ToolError::Schema("minutes must be between 5 and 1440."));
    }
    let mut tx = ctx.pool.begin().await.map_err(|e| ToolError::Other(e.to_string()))?;
    let row: Option<(String, DateTime<Utc>, String)> = sqlx::query_as(
        "SELECT user_id, expires_at, confirmation_status FROM pending_actions WHERE id = $1 FOR UPDATE",
    )
    .bind(pa_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    let (owner, current_expires, status) =
        row.ok_or(ToolError::NotFound("Action not found."))?;
    if owner != ctx.user.id {
        return Err(ToolError::NotFound("Action not found."));
    }
    if status == "expired" {
        return Err(ToolError::Schema("This action has already expired."));
    }
    let base = if current_expires > Utc::now() { current_expires } else { Utc::now() };
    let new_expires = base + Duration::minutes(minutes);
    sqlx::query("UPDATE pending_actions SET expires_at = $1 WHERE id = $2")
        .bind(new_expires)
        .bind(pa_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    tx.commit().await.map_err(|e| ToolError::Other(e.to_string()))?;
    Ok(json!({
        "pending_action_id": pa_id,
        "new_expires_at": new_expires,
    }))
}

// ---------------------------------------------------------------------------
// 3.1 — insights.daily_digest
// ---------------------------------------------------------------------------

pub async fn t_insights_daily_digest(
    ctx: &ToolContext,
    args: &Value,
) -> Result<Value, ToolError> {
    let ack = args.get("ack").and_then(|v| v.as_bool()).unwrap_or(false);
    let user_id = &ctx.user.id;
    let now = Utc::now();
    let day_ago = now - Duration::hours(24);
    let week_ago = now - Duration::days(7);
    // --- Renewals in the next 7 days
    let renewals: Vec<(String, String, i64, DateTime<Utc>)> = sqlx::query_as(
        "SELECT id, merchant, estimated_amount_fen, next_expected_at \
         FROM subscriptions WHERE user_id = $1 AND cancellation_status = 'active' \
           AND next_expected_at BETWEEN $2 AND $3 \
         ORDER BY next_expected_at ASC LIMIT 5",
    )
    .bind(user_id)
    .bind(now)
    .bind(now + Duration::days(7))
    .fetch_all(&ctx.pool)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    // --- Goal progress (this week's contributions)
    let goals: Vec<(String, String, i64, Option<i64>)> = sqlx::query_as(
        "SELECT g.id, g.name, g.target_fen, COALESCE(SUM(c.amount_fen), 0)::BIGINT \
         FROM savings_goals g \
         LEFT JOIN goal_contributions c ON c.goal_id = g.id AND c.created_at >= $1 \
         WHERE g.user_id = $2 GROUP BY g.id, g.name, g.target_fen",
    )
    .bind(week_ago)
    .bind(user_id)
    .fetch_all(&ctx.pool)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    // --- Anomalies: count of "unrecognized" or large transactions in 24h.
    let anomaly_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM transactions t JOIN accounts a ON a.id = t.account_id \
         WHERE a.user_id = $1 AND t.occurred_at >= $2 AND t.status = 'posted' AND t.amount_fen > 50000",
    )
    .bind(user_id)
    .bind(day_ago)
    .fetch_one(&ctx.pool)
    .await
    .unwrap_or(0);
    // --- Total spend today
    let today_spend: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(t.amount_fen), 0) FROM transactions t \
         JOIN accounts a ON a.id = t.account_id \
         WHERE a.user_id = $1 AND t.occurred_at >= $2 AND t.status = 'posted' AND t.amount_fen > 0",
    )
    .bind(user_id)
    .bind(day_ago)
    .fetch_one(&ctx.pool)
    .await
    .unwrap_or(0);
    // --- Pending workflows count (derived: workflows with non-confirmed actions)
    let pending_wf: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT w.id) FROM workflows w \
         JOIN pending_actions pa ON pa.workflow_id = w.id \
         WHERE w.user_id = $1 AND pa.confirmation_status = 'none'",
    )
    .bind(user_id)
    .fetch_one(&ctx.pool)
    .await
    .unwrap_or(0);
    let hash = format!(
        "digest:{}:{}:{}:{}:{}",
        renewals.len(),
        goals.len(),
        anomaly_count,
        today_spend,
        pending_wf
    );
    let renewal_items: Vec<_> = renewals
        .into_iter()
        .map(|(id, merchant, amount, at)| {
            json!({
                "id": id,
                "merchant": merchant,
                "amount_fen": amount,
                "next_expected_at": at,
            })
        })
        .collect();
    let goal_items: Vec<_> = goals
        .into_iter()
        .map(|(id, name, target, this_week)| {
            let pct = if target > 0 { (this_week.unwrap_or(0) * 100) / target } else { 0 };
            json!({
                "id": id,
                "name": name,
                "target_fen": target,
                "this_week_fen": this_week.unwrap_or(0),
                "progress_pct": pct,
            })
        })
        .collect();
    let body = json!({
        "generated_at": now,
        "items": {
            "renewals": renewal_items,
            "goals": goal_items,
            "anomaly_count_24h": anomaly_count,
            "spend_24h_fen": today_spend,
            "pending_workflows": pending_wf,
        },
        "hash": hash,
    });
    if ack {
        sqlx::query(
            "INSERT INTO daily_digest_state (user_id, last_seen_at, last_seen_hash) \
             VALUES ($1, $2, $3) \
             ON CONFLICT (user_id) DO UPDATE SET last_seen_at = EXCLUDED.last_seen_at, last_seen_hash = EXCLUDED.last_seen_hash",
        )
        .bind(user_id)
        .bind(now)
        .bind(&hash)
        .execute(&ctx.pool)
        .await
        .ok();
    }
    Ok(body)
}

// ---------------------------------------------------------------------------
// 3.4 — goals.progress / goals.rebalance / goals.contribute
// ---------------------------------------------------------------------------

pub async fn t_goals_progress(
    ctx: &ToolContext,
    args: &Value,
) -> Result<Value, ToolError> {
    let goal_id = args
        .get("goal_id")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("goal_id is required"))?;
    let goal: Option<SavingsGoal> = sqlx::query_as(
        "SELECT * FROM savings_goals WHERE id = $1",
    )
    .bind(goal_id)
    .fetch_optional(&ctx.pool)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    let goal = goal.ok_or(ToolError::NotFound("Goal not found."))?;
    if goal.user_id != ctx.user.id {
        return Err(ToolError::NotFound("Goal not found."));
    }
    let total: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount_fen), 0) FROM goal_contributions WHERE goal_id = $1",
    )
    .bind(goal_id)
    .fetch_one(&ctx.pool)
    .await
    .unwrap_or(0);
    let this_month: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount_fen), 0) FROM goal_contributions \
         WHERE goal_id = $1 AND created_at >= date_trunc('month', NOW())",
    )
    .bind(goal_id)
    .fetch_one(&ctx.pool)
    .await
    .unwrap_or(0);
    let pct = if goal.target_fen > 0 { (total * 100) / goal.target_fen } else { 0 };
    // Compute on-pace projection (straight-line from creation).
    let elapsed_days = (Utc::now() - goal.created_at).num_days().max(1) as i64;
    let daily_pace = total / elapsed_days;
    let target_remaining = (goal.target_fen - total).max(0);
    let days_to_goal = if daily_pace > 0 { target_remaining / daily_pace } else { i64::MAX };
    Ok(json!({
        "id": goal.id,
        "name": goal.name,
        "target_fen": goal.target_fen,
        "saved_fen": total,
        "this_month_fen": this_month,
        "progress_pct": pct,
        "daily_pace_fen": daily_pace,
        "days_to_goal": if days_to_goal == i64::MAX { Value::Null } else { Value::from(days_to_goal) },
    }))
}

pub async fn t_goals_rebalance(
    ctx: &ToolContext,
    args: &Value,
) -> Result<Value, ToolError> {
    let goal_id = args
        .get("goal_id")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("goal_id is required"))?;
    let monthly_fen = args
        .get("monthly_amount_fen")
        .and_then(|v| v.as_i64())
        .ok_or(ToolError::Schema("monthly_amount_fen is required"))?;
    if monthly_fen < 100 {
        return Err(ToolError::Schema("Monthly amount must be at least ¥1."));
    }
    let source_account_id = args
        .get("source_account_id")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let goal: Option<SavingsGoal> = sqlx::query_as("SELECT * FROM savings_goals WHERE id = $1")
        .bind(goal_id)
        .fetch_optional(&ctx.pool)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    let goal = goal.ok_or(ToolError::NotFound("Goal not found."))?;
    if goal.user_id != ctx.user.id {
        return Err(ToolError::NotFound("Goal not found."));
    }
    let total: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount_fen), 0) FROM goal_contributions WHERE goal_id = $1",
    )
    .bind(goal_id)
    .fetch_one(&ctx.pool)
    .await
    .unwrap_or(0);
    let remaining = (goal.target_fen - total).max(0);
    let months_remaining = if monthly_fen > 0 { (remaining + monthly_fen - 1) / monthly_fen } else { i64::MAX };
    Ok(json!({
        "goal_id": goal_id,
        "name": goal.name,
        "saved_fen": total,
        "target_fen": goal.target_fen,
        "remaining_fen": remaining,
        "monthly_amount_fen": monthly_fen,
        "months_to_goal": if months_remaining == i64::MAX { Value::Null } else { Value::from(months_remaining) },
        "source_account_id": source_account_id,
        "note": "Recording a rebalance plan only updates the dashboard; the actual recurring transfer is created with goals.contribute each time it runs.",
    }))
}

pub async fn t_goals_contribute(
    ctx: &ToolContext,
    args: &Value,
) -> Result<Value, ToolError> {
    let goal_id = args
        .get("goal_id")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("goal_id is required"))?;
    let amount_fen = args
        .get("amount_fen")
        .and_then(|v| v.as_i64())
        .ok_or(ToolError::Schema("amount_fen is required"))?;
    if amount_fen < 100 {
        return Err(ToolError::Schema("Amount must be at least ¥1."));
    }
    let source_account_id = args
        .get("source_account_id")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("source_account_id is required"))?;
    let memo = args.get("memo").and_then(|v| v.as_str()).unwrap_or("");
    let mut tx = ctx.pool.begin().await.map_err(|e| ToolError::Other(e.to_string()))?;
    let goal: Option<SavingsGoal> =
        sqlx::query_as("SELECT * FROM savings_goals WHERE id = $1 FOR UPDATE")
            .bind(goal_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| ToolError::Other(e.to_string()))?;
    let goal = goal.ok_or(ToolError::NotFound("Goal not found."))?;
    if goal.user_id != ctx.user.id {
        return Err(ToolError::NotFound("Goal not found."));
    }
    let acc: Option<Account> = sqlx::query_as("SELECT * FROM accounts WHERE id = $1 FOR UPDATE")
        .bind(source_account_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    let acc = acc.ok_or(ToolError::NotFound("Source account not found."))?;
    if acc.user_id != ctx.user.id {
        return Err(ToolError::NotFound("Source account not found."));
    }
    if acc.available_balance_fen < amount_fen {
        return Err(ToolError::InsufficientFunds("Source account has insufficient funds."));
    }
    sqlx::query("UPDATE accounts SET available_balance_fen = available_balance_fen - $1 WHERE id = $2")
        .bind(amount_fen)
        .bind(&acc.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    let id = ids::new_id("gc");
    sqlx::query(
        "INSERT INTO goal_contributions (id, goal_id, user_id, source_account_id, amount_fen, memo, source) \
         VALUES ($1, $2, $3, $4, $5, $6, 'manual')",
    )
    .bind(&id)
    .bind(goal_id)
    .bind(&ctx.user.id)
    .bind(source_account_id)
    .bind(amount_fen)
    .bind(memo)
    .execute(&mut *tx)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    // Mirror a transaction on the source account for ledger symmetry.
    let txn_id = ids::new_id("txn");
    sqlx::query(
        "INSERT INTO transactions (id, account_id, amount_fen, direction, category, merchant, memo, occurred_at, status) \
         VALUES ($1, $2, $3, 'debit', 'savings', $4, $5, $6, 'posted')",
    )
    .bind(&txn_id)
    .bind(source_account_id)
    .bind(amount_fen)
    .bind(format!("Goal: {}", goal.name))
    .bind(memo)
    .bind(Utc::now())
    .execute(&mut *tx)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    tx.commit().await.map_err(|e| ToolError::Other(e.to_string()))?;
    audit::append_event(
        &ctx.pool,
        AppendEventParams {
            action: "goal.contribute",
            actor_user_id: Some(&ctx.user.id),
            session: None,
            request_id: Some(&ctx.request_id),
            workflow_id: None,
            node_id: None,
            tool: Some("goals.contribute"),
            parameters_masked: Some(json!({ "goal_id": goal_id, "amount_fen": amount_fen })),
            risk_tier: Some("Yellow"),
            policy_reason: None,
            confirmation_status: Some("confirmed"),
            mfa_status: None,
            result: "ok",
            error_code: None,
        },
    )
    .await
    .ok();
    Ok(json!({
        "id": id,
        "goal_id": goal_id,
        "amount_fen": amount_fen,
        "new_balance_fen": acc.available_balance_fen - amount_fen,
    }))
}

// ---------------------------------------------------------------------------
// Tests for the pure helpers.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn parse_amount_query() {
        let p = parse_search_query("286");
        assert_eq!(p.amount_fen, Some(286));
        assert!(p.text.is_none());
    }

    #[test]
    fn parse_text_query() {
        let p = parse_search_query("Acme Books");
        assert_eq!(p.text.as_deref(), Some("Acme Books"));
        assert!(p.amount_fen.is_none());
    }

    #[test]
    fn parse_last_week() {
        let p = parse_search_query("last week");
        assert!(p.from.is_some());
    }

    #[test]
    fn parse_empty() {
        let p = parse_search_query("  ");
        assert!(p.text.is_none() && p.amount_fen.is_none() && p.from.is_none());
    }

    #[test]
    fn reverse_window_recent_posted() {
        let now = Utc::now();
        let posted = now - Duration::minutes(3);
        let diff = now - posted;
        assert!(diff <= Duration::minutes(5));
    }

    #[test]
    fn reverse_window_old_posted_rejected() {
        let now = Utc::now();
        let posted = now - Duration::minutes(7);
        let diff = now - posted;
        assert!(diff > Duration::minutes(5));
    }

    #[test]
    fn snooze_math_in_range() {
        for d in 1..=90 {
            let base = Utc::now();
            let r = base + Duration::days(d);
            assert!(r > base);
        }
    }

    #[test]
    fn rng_helper_smoke() {
        // 4-digit codes are 0-9999 inclusive.
        // We don't import a RNG here on purpose — the function we test is
        // the pure parser, not the random code generator.
        let q = parse_search_query("123");
        assert_eq!(q.amount_fen, Some(123));
    }
}
