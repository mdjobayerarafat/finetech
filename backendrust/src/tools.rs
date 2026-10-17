//! Tool registry and executor — the fixed allowlist of operations
//! the system can perform.
//!
//! Each tool takes a `ToolContext` and a parameter map, mutates state
//! in a single transaction, and returns a serialisable result map.

use chrono::{DateTime, Utc};
use rand::Rng;
use serde_json::{json, Value};
use sqlx::PgPool;

use crate::audit::{self, AppendEventParams};
use crate::ids;
use crate::models::{Account, Budget, BudgetReservation, Card, Payee, Reminder, Reservation, SavingsGoal, Subscription, Transaction, User, WealthHolding, WealthProduct};

#[derive(thiserror::Error, Debug)]
pub enum ToolError {
    #[error("{0}")]
    Schema(&'static str),
    #[error("{0}")]
    NotFound(&'static str),
    #[error("{0}")]
    InsufficientFunds(&'static str),
    #[error("{0}")]
    BelowMinimum(&'static str),
    #[error("{0}")]
    Clarification(String),
    /// Hard refusal by an admin-configured rule. The user-facing message
    /// is suitable for chat — no internal codes or stack traces.
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    Other(String),
}

impl ToolError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Schema(_) => "schema_invalid",
            Self::NotFound(_) => "not_found",
            Self::InsufficientFunds(_) => "insufficient_funds",
            Self::BelowMinimum(_) => "below_minimum",
            Self::Clarification(_) => "clarification_required",
            Self::Forbidden(_) => "forbidden",
            Self::Other(_) => "schema_invalid",
        }
    }
}

impl From<sqlx::Error> for ToolError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => ToolError::NotFound("Row not found"),
            other => ToolError::Other(other.to_string()),
        }
    }
}

#[derive(Clone)]
pub struct ToolContext {
    pub pool: PgPool,
    pub user: User,
    pub request_id: String,
    pub session_id: Option<String>,
}

/// Borrow a connection from the pool and run `f` inside a single tx.
// (Tool functions use their own transactions inline.)

fn now() -> DateTime<Utc> {
    Utc::now()
}

fn rand4() -> String {
    let mut r = rand::thread_rng();
    format!("{:04}", r.gen_range(0..10000))
}

// --- Tool implementations -------------------------------------------------

pub async fn t_accounts_balance_check(
    ctx: &ToolContext,
    params: &Value,
) -> Result<Value, ToolError> {
    let account_id = params
        .get("account_id")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("account_id is required"))?;
    if !account_id.starts_with("acc_") {
        return Err(ToolError::Schema("account_id must be an account id"));
    }
    let mut conn = ctx
        .pool
        .acquire()
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    let acc: Option<Account> = sqlx::query_as("SELECT * FROM accounts WHERE id = $1 AND user_id = $2 FOR UPDATE")
        .bind(account_id)
        .bind(&ctx.user.id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    let acc = acc.ok_or(ToolError::NotFound("Account not found"))?;
    Ok(json!({
        "account_id": acc.id,
        "available_balance_fen": acc.available_balance_fen,
        "reserved_balance_fen": acc.reserved_balance_fen,
    }))
}

pub async fn t_profile_resolve_date(
    _ctx: &ToolContext,
    params: &Value,
) -> Result<Value, ToolError> {
    let date = params
        .get("date")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("date must be a string"))?;
    if date.len() < 8 {
        return Err(ToolError::Schema("date must be at least 8 chars"));
    }
    Ok(json!({ "date": date }))
}

pub async fn t_transfer_create_pending(
    ctx: &ToolContext,
    params: &Value,
) -> Result<Value, ToolError> {
    let account_id = params
        .get("account_id")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("account_id required"))?;
    let payee_id = params
        .get("payee_id")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("payee_id required"))?;
    let amount_fen = params
        .get("amount_fen")
        .and_then(|v| v.as_i64())
        .ok_or(ToolError::Schema("amount_fen required"))?;
    if amount_fen <= 0 {
        return Err(ToolError::Schema("amount_fen must be > 0"));
    }
    let memo = params
        .get("memo")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let schedule: Option<DateTime<Utc>> = params
        .get("schedule")
        .and_then(|v| v.as_str())
        .and_then(|s| {
            let s = s.replace("Z", "+00:00");
            DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc))
        });
    let idempotency_key = params
        .get("idempotency_key")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("idempotency_key required"))?
        .to_string();

    let pool = ctx.pool.clone();
    let user_id = ctx.user.id.clone();
    let request_id = ctx.request_id.clone();

    let result: Result<Value, ToolError> = (async {
        let mut tx = pool
            .begin()
            .await
            .map_err(|e| ToolError::Other(e.to_string()))?;
        let mut account =
            lock_account(&mut *tx, account_id).await?;
        if account.user_id != user_id {
            return Err(ToolError::NotFound("Account not found"));
        }
        if account.available_balance_fen < amount_fen {
            return Err(ToolError::InsufficientFunds(
                "Source account does not have enough available balance.",
            ));
        }
        let payee = resolve_payee(&mut *tx, &user_id, payee_id).await?;
        let transfer_status = if schedule.is_some() { "scheduled" } else { "posted" };
        let transfer_id = ids::new_id("xfer");
        let n = now();
        sqlx::query(
            "INSERT INTO transfers (id, source_account_id, payee_id, amount_fen, memo, schedule, idempotency_key, status, posted_at, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(&transfer_id)
        .bind(&account.id)
        .bind(&payee.id)
        .bind(amount_fen)
        .bind(&memo)
        .bind(schedule)
        .bind(&idempotency_key)
        .bind(transfer_status)
        .bind(if schedule.is_some() { None } else { Some(n) })
        .bind(n)
        .execute(&mut *tx)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;

        if payee.first_used_at.is_none() {
            sqlx::query("UPDATE payees SET first_used_at = $1 WHERE id = $2")
                .bind(n)
                .bind(&payee.id)
                .execute(&mut *tx)
                .await
                .map_err(|e| ToolError::Other(e.to_string()))?;
        }

        // Debit ledger entry.
        let debit = crate::models::Transaction {
            id: ids::new_id("tx"),
            account_id: account.id.clone(),
            amount_fen: -amount_fen,
            direction: "debit".to_string(),
            category: "transfer".to_string(),
            merchant: payee.display_name.clone(),
            memo: memo.clone(),
            occurred_at: n,
            status: "posted".to_string(),
        };
        sqlx::query(
            "INSERT INTO transactions (id, account_id, amount_fen, direction, category, merchant, memo, occurred_at, status)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(&debit.id)
        .bind(&debit.account_id)
        .bind(debit.amount_fen)
        .bind(&debit.direction)
        .bind(&debit.category)
        .bind(&debit.merchant)
        .bind(&debit.memo)
        .bind(debit.occurred_at)
        .bind(&debit.status)
        .execute(&mut *tx)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;

        sqlx::query("UPDATE accounts SET available_balance_fen = available_balance_fen - $1 WHERE id = $2")
            .bind(amount_fen)
            .bind(&account.id)
            .execute(&mut *tx)
            .await
            .map_err(|e| ToolError::Other(e.to_string()))?;
        account.available_balance_fen -= amount_fen;

        // Audit event.
        let mut masked = serde_json::Map::new();
        masked.insert("payee_id".into(), Value::String(payee.id.clone()));
        masked.insert("amount_fen".into(), Value::Number(amount_fen.into()));
        masked.insert("memo".into(), Value::String(memo.chars().take(64).collect()));
        audit::append_event(
            &pool,
            AppendEventParams {
                action: "transfer.execute",
                actor_user_id: Some(&user_id),
                session: None,
                request_id: Some(&request_id),
                workflow_id: None,
                node_id: None,
                tool: Some("transfer.create_pending"),
                parameters_masked: Some(Value::Object(masked)),
                risk_tier: params
                    .get("tier")
                    .and_then(|v| v.as_str()),
                policy_reason: None,
                confirmation_status: None,
                mfa_status: None,
                result: "ok",
                error_code: None,
            },
        )
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| ToolError::Other(e.to_string()))?;
        Ok(json!({
            "transfer_id": transfer_id,
            "status": transfer_status,
            "amount_fen": amount_fen,
            "posted_at": if schedule.is_some() { Value::Null } else { Value::String(n.to_rfc3339()) },
        }))
    })
    .await;
    result
}

pub async fn t_subscriptions_cancel(
    ctx: &ToolContext,
    params: &Value,
) -> Result<Value, ToolError> {
    let subscription_id = params
        .get("subscription_id")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("subscription_id required"))?;
    let mut tx = ctx
        .pool
        .begin()
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    let sub: Option<crate::models::Subscription> =
        sqlx::query_as("SELECT id, user_id, merchant, estimated_amount_fen, interval_days, next_expected_at, last_charge_at, evidence::text AS evidence, cancellation_status, cancelled_at, created_at FROM subscriptions WHERE id = $1")
            .bind(subscription_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| ToolError::Other(e.to_string()))?;
    let sub = sub.ok_or(ToolError::NotFound("Subscription not found"))?;
    if sub.user_id != ctx.user.id {
        return Err(ToolError::NotFound("Subscription not found"));
    }
    let n = now();
    let mut _status = sub.cancellation_status.clone();
    if !matches!(
        _status.as_str(),
        "cancellation_requested" | "cancelled"
    ) {
        sqlx::query(
            "UPDATE subscriptions SET cancellation_status = 'cancellation_requested', cancelled_at = $1 WHERE id = $2",
        )
        .bind(n)
        .bind(&sub.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
        // Mock merchant connector — 100% success in the demo.
        sqlx::query("UPDATE subscriptions SET cancellation_status = 'cancelled' WHERE id = $1")
            .bind(&sub.id)
            .execute(&mut *tx)
            .await
            .map_err(|e| ToolError::Other(e.to_string()))?;
        _status = "cancelled".to_string();
    }
    audit::append_event(
        &ctx.pool,
        AppendEventParams {
            action: "subscription.cancel",
            actor_user_id: Some(&ctx.user.id),
            session: None,
            request_id: Some(&ctx.request_id),
            workflow_id: None,
            node_id: None,
            tool: Some("subscriptions.cancel"),
            parameters_masked: Some(json!({
                "subscription_id": sub.id,
                "merchant": sub.merchant,
            })),
            risk_tier: params
                .get("tier")
                .and_then(|v| v.as_str()),
            policy_reason: None,
            confirmation_status: None,
            mfa_status: None,
            result: "ok",
            error_code: None,
        },
    )
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    tx.commit()
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    Ok(json!({
        "subscription_id": sub.id,
        "status": _status,
    }))
}

pub async fn t_subscriptions_remind(
    ctx: &ToolContext,
    params: &Value,
) -> Result<Value, ToolError> {
    let subscription_id = params
        .get("subscription_id")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("subscription_id required"))?;
    let days_before: i64 = params
        .get("days_before")
        .and_then(|v| v.as_i64())
        .unwrap_or(3);

    let mut tx = ctx
        .pool
        .begin()
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    let sub: Option<crate::models::Subscription> =
        sqlx::query_as("SELECT id, user_id, merchant, estimated_amount_fen, interval_days, next_expected_at, last_charge_at, evidence::text AS evidence, cancellation_status, cancelled_at, created_at FROM subscriptions WHERE id = $1")
            .bind(subscription_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| ToolError::Other(e.to_string()))?;
    let sub = sub.ok_or(ToolError::NotFound("Subscription not found"))?;
    if sub.user_id != ctx.user.id {
        return Err(ToolError::NotFound("Subscription not found"));
    }
    let due = sub.next_expected_at;
    let remind_at = due - chrono::Duration::days(days_before);

    let reminder = Reminder {
        id: ids::new_id("rem"),
        user_id: ctx.user.id.clone(),
        kind: "renewal".to_string(),
        subject: format!("Subscription '{}' renews soon", sub.merchant),
        due_at: remind_at,
        status: "scheduled".to_string(),
        linked_subscription_id: Some(sub.id.clone()),
        created_at: now(),
    };
    sqlx::query(
        "INSERT INTO reminders (id, user_id, kind, subject, due_at, status, linked_subscription_id, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(&reminder.id)
    .bind(&reminder.user_id)
    .bind(&reminder.kind)
    .bind(&reminder.subject)
    .bind(reminder.due_at)
    .bind(&reminder.status)
    .bind(&reminder.linked_subscription_id)
    .bind(reminder.created_at)
    .execute(&mut *tx)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;

    audit::append_event(
        &ctx.pool,
        AppendEventParams {
            action: "subscription.remind",
            actor_user_id: Some(&ctx.user.id),
            session: None,
            request_id: Some(&ctx.request_id),
            workflow_id: None,
            node_id: None,
            tool: Some("subscriptions.remind"),
            parameters_masked: Some(json!({
                "subscription_id": sub.id,
                "days_before": days_before,
            })),
            risk_tier: Some("Green"),
            policy_reason: None,
            confirmation_status: None,
            mfa_status: None,
            result: "ok",
            error_code: None,
        },
    )
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    tx.commit()
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    Ok(json!({
        "reminder_id": reminder.id,
        "due_at": remind_at.to_rfc3339(),
    }))
}

pub async fn t_goals_create_reservation(
    ctx: &ToolContext,
    params: &Value,
) -> Result<Value, ToolError> {
    let goal_id = params
        .get("goal_id")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("goal_id required"))?;
    let amount_fen = params
        .get("amount_fen")
        .and_then(|v| v.as_i64())
        .ok_or(ToolError::Schema("amount_fen required"))?;
    let account_id = params
        .get("account_id")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("account_id required"))?;
    if amount_fen <= 0 {
        return Err(ToolError::Schema("amount_fen must be > 0"));
    }

    let mut tx = ctx
        .pool
        .begin()
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    let mut account = lock_account(&mut *tx, account_id).await?;
    if account.user_id != ctx.user.id {
        return Err(ToolError::NotFound("Account not found"));
    }
    let goal: Option<SavingsGoal> = sqlx::query_as("SELECT * FROM savings_goals WHERE id = $1")
        .bind(goal_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    let goal = goal.ok_or(ToolError::NotFound("Goal not found"))?;
    if goal.user_id != ctx.user.id {
        return Err(ToolError::NotFound("Goal not found"));
    }
    if account.available_balance_fen < amount_fen {
        return Err(ToolError::InsufficientFunds("Not enough available balance"));
    }
    sqlx::query("UPDATE accounts SET available_balance_fen = available_balance_fen - $1, reserved_balance_fen = reserved_balance_fen + $2 WHERE id = $3")
        .bind(amount_fen)
        .bind(amount_fen)
        .bind(&account.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    account.available_balance_fen -= amount_fen;
    account.reserved_balance_fen += amount_fen;
    let res = Reservation {
        id: ids::new_id("res"),
        goal_id: goal.id.clone(),
        amount_fen,
        state: "held".to_string(),
        source_account_id: account.id.clone(),
        created_at: now(),
    };
    sqlx::query(
        "INSERT INTO reservations (id, goal_id, amount_fen, state, source_account_id, created_at) VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(&res.id)
    .bind(&res.goal_id)
    .bind(res.amount_fen)
    .bind(&res.state)
    .bind(&res.source_account_id)
    .bind(res.created_at)
    .execute(&mut *tx)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    audit::append_event(
        &ctx.pool,
        AppendEventParams {
            action: "goal.reserve",
            actor_user_id: Some(&ctx.user.id),
            session: None,
            request_id: Some(&ctx.request_id),
            workflow_id: None,
            node_id: None,
            tool: Some("goals.create_reservation"),
            parameters_masked: Some(json!({
                "goal_id": goal.id,
                "amount_fen": amount_fen,
            })),
            risk_tier: params.get("tier").and_then(|v| v.as_str()),
            policy_reason: None,
            confirmation_status: None,
            mfa_status: None,
            result: "ok",
            error_code: None,
        },
    )
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    tx.commit()
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    Ok(json!({
        "reservation_id": res.id,
        "state": res.state,
    }))
}

pub async fn t_goals_consume_reservation(
    ctx: &ToolContext,
    params: &Value,
) -> Result<Value, ToolError> {
    let reservation_id = params
        .get("reservation_id")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("reservation_id required"))?;
    let mut tx = ctx
        .pool
        .begin()
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    let res: Option<Reservation> = sqlx::query_as("SELECT * FROM reservations WHERE id = $1")
        .bind(reservation_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    let res = res.ok_or(ToolError::NotFound("Reservation not found"))?;
    let goal: Option<SavingsGoal> = sqlx::query_as("SELECT * FROM savings_goals WHERE id = $1")
        .bind(&res.goal_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    let goal = goal.ok_or(ToolError::NotFound("Goal not found"))?;
    if goal.user_id != ctx.user.id {
        return Err(ToolError::NotFound("Goal not found"));
    }
    let mut account = lock_account(&mut *tx, &res.source_account_id).await?;
    if account.user_id != ctx.user.id {
        return Err(ToolError::NotFound("Account not found"));
    }
    if res.state != "held" {
        tx.commit()
            .await
            .map_err(|e| ToolError::Other(e.to_string()))?;
        return Ok(json!({ "reservation_id": res.id, "state": res.state }));
    }
    sqlx::query("UPDATE reservations SET state = 'consumed' WHERE id = $1")
        .bind(&res.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    sqlx::query("UPDATE accounts SET reserved_balance_fen = reserved_balance_fen - $1 WHERE id = $2")
        .bind(res.amount_fen)
        .bind(&account.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    account.reserved_balance_fen -= res.amount_fen;
    audit::append_event(
        &ctx.pool,
        AppendEventParams {
            action: "goal.consume",
            actor_user_id: Some(&ctx.user.id),
            session: None,
            request_id: Some(&ctx.request_id),
            workflow_id: None,
            node_id: None,
            tool: Some("goals.consume_reservation"),
            parameters_masked: Some(json!({
                "reservation_id": res.id,
                "amount_fen": res.amount_fen,
            })),
            risk_tier: params.get("tier").and_then(|v| v.as_str()),
            policy_reason: None,
            confirmation_status: None,
            mfa_status: None,
            result: "ok",
            error_code: None,
        },
    )
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    tx.commit()
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    Ok(json!({
        "reservation_id": res.id,
        "state": "consumed",
    }))
}

pub async fn t_goals_release_reservation(
    ctx: &ToolContext,
    params: &Value,
) -> Result<Value, ToolError> {
    let reservation_id = params
        .get("reservation_id")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("reservation_id required"))?;
    let mut tx = ctx
        .pool
        .begin()
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    let res: Option<Reservation> = sqlx::query_as("SELECT * FROM reservations WHERE id = $1")
        .bind(reservation_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    let res = res.ok_or(ToolError::NotFound("Reservation not found"))?;
    let goal: Option<SavingsGoal> = sqlx::query_as("SELECT * FROM savings_goals WHERE id = $1")
        .bind(&res.goal_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    let goal = goal.ok_or(ToolError::NotFound("Goal not found"))?;
    if goal.user_id != ctx.user.id {
        return Err(ToolError::NotFound("Goal not found"));
    }
    let mut account = lock_account(&mut *tx, &res.source_account_id).await?;
    if account.user_id != ctx.user.id {
        return Err(ToolError::NotFound("Account not found"));
    }
    if res.state != "held" {
        tx.commit()
            .await
            .map_err(|e| ToolError::Other(e.to_string()))?;
        return Ok(json!({ "reservation_id": res.id, "state": res.state }));
    }
    sqlx::query("UPDATE reservations SET state = 'released' WHERE id = $1")
        .bind(&res.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    sqlx::query("UPDATE accounts SET reserved_balance_fen = reserved_balance_fen - $1, available_balance_fen = available_balance_fen + $2 WHERE id = $3")
        .bind(res.amount_fen)
        .bind(res.amount_fen)
        .bind(&account.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    account.reserved_balance_fen -= res.amount_fen;
    account.available_balance_fen += res.amount_fen;
    audit::append_event(
        &ctx.pool,
        AppendEventParams {
            action: "goal.release",
            actor_user_id: Some(&ctx.user.id),
            session: None,
            request_id: Some(&ctx.request_id),
            workflow_id: None,
            node_id: None,
            tool: Some("goals.release_reservation"),
            parameters_masked: Some(json!({
                "reservation_id": res.id,
                "amount_fen": res.amount_fen,
            })),
            risk_tier: params.get("tier").and_then(|v| v.as_str()),
            policy_reason: None,
            confirmation_status: None,
            mfa_status: None,
            result: "ok",
            error_code: None,
        },
    )
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    tx.commit()
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    Ok(json!({
        "reservation_id": res.id,
        "state": "released",
    }))
}

pub async fn t_reminders_create(
    ctx: &ToolContext,
    params: &Value,
) -> Result<Value, ToolError> {
    let subject = params
        .get("subject")
        .and_then(|v| v.as_str())
        .unwrap_or("Reminder");
    let due_at_str = params
        .get("due_at")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("due_at required"))?;
    let due_at: DateTime<Utc> = DateTime::parse_from_rfc3339(due_at_str.replace("Z", "+00:00").as_str())
        .map_err(|_| ToolError::Schema("due_at invalid"))?
        .with_timezone(&Utc);

    let mut tx = ctx
        .pool
        .begin()
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    let reminder = Reminder {
        id: ids::new_id("rem"),
        user_id: ctx.user.id.clone(),
        kind: "custom".to_string(),
        subject: subject.to_string(),
        due_at,
        status: "scheduled".to_string(),
        linked_subscription_id: None,
        created_at: now(),
    };
    sqlx::query(
        "INSERT INTO reminders (id, user_id, kind, subject, due_at, status, linked_subscription_id, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(&reminder.id)
    .bind(&reminder.user_id)
    .bind(&reminder.kind)
    .bind(&reminder.subject)
    .bind(reminder.due_at)
    .bind(&reminder.status)
    .bind(&reminder.linked_subscription_id)
    .bind(reminder.created_at)
    .execute(&mut *tx)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    audit::append_event(
        &ctx.pool,
        AppendEventParams {
            action: "reminder.create",
            actor_user_id: Some(&ctx.user.id),
            session: None,
            request_id: Some(&ctx.request_id),
            workflow_id: None,
            node_id: None,
            tool: Some("reminders.create"),
            parameters_masked: Some(json!({
                "subject": subject.chars().take(64).collect::<String>(),
                "due_at": due_at.to_rfc3339(),
            })),
            risk_tier: Some("Green"),
            policy_reason: None,
            confirmation_status: None,
            mfa_status: None,
            result: "ok",
            error_code: None,
        },
    )
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    tx.commit()
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    Ok(json!({ "reminder_id": reminder.id }))
}

pub async fn t_connectors_merchant_order(
    ctx: &ToolContext,
    params: &Value,
) -> Result<Value, ToolError> {
    let merchant = params
        .get("merchant")
        .and_then(|v| v.as_str())
        .unwrap_or("mock-merchant");
    let amount_fen = params
        .get("amount_fen")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let deliver_at = params
        .get("deliver_at")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    audit::append_event(
        &ctx.pool,
        AppendEventParams {
            action: "connector.merchant_order",
            actor_user_id: Some(&ctx.user.id),
            session: None,
            request_id: Some(&ctx.request_id),
            workflow_id: None,
            node_id: None,
            tool: Some("connectors.merchant.order"),
            parameters_masked: Some(json!({
                "merchant": merchant,
                "amount_fen": amount_fen,
                "deliver_at": deliver_at,
            })),
            risk_tier: params.get("tier").and_then(|v| v.as_str()),
            policy_reason: None,
            confirmation_status: None,
            mfa_status: None,
            result: "ok",
            error_code: None,
        },
    )
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    Ok(json!({
        "simulation": true,
        "merchant": merchant,
        "amount_fen": amount_fen,
        "deliver_at": deliver_at,
    }))
}

pub async fn t_cards_action(
    ctx: &ToolContext,
    params: &Value,
) -> Result<Value, ToolError> {
    let kind = params
        .get("kind")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("kind required"))?;
    let card_id = params
        .get("card_id")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("card_id required"))?;

    let mut tx = ctx
        .pool
        .begin()
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    let card: Option<crate::models::Card> = sqlx::query_as("SELECT * FROM cards WHERE id = $1")
        .bind(card_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    let card = card.ok_or(ToolError::NotFound("Card not found"))?;
    if card.user_id != ctx.user.id {
        return Err(ToolError::NotFound("Card not found"));
    }

    if kind == "issue" {
        return Err(ToolError::Schema("use cards.issue for new cards"));
    }
    let new_status = match kind {
        "freeze" => "frozen",
        "unfreeze" => "active",
        "cancel" => "cancelled",
        "report_lost" => "lost",
        _ => return Err(ToolError::Schema("unknown card action")),
    };
    sqlx::query("UPDATE cards SET status = $1 WHERE id = $2")
        .bind(new_status)
        .bind(&card.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    if kind == "update_limits" {
        if let Some(v) = params.get("single_tx_limit_fen").and_then(|v| v.as_i64()) {
            sqlx::query("UPDATE cards SET single_tx_limit_fen = $1 WHERE id = $2")
                .bind(v)
                .bind(&card.id)
                .execute(&mut *tx)
                .await
                .map_err(|e| ToolError::Other(e.to_string()))?;
        }
        if let Some(v) = params.get("monthly_limit_fen").and_then(|v| v.as_i64()) {
            sqlx::query("UPDATE cards SET monthly_limit_fen = $1 WHERE id = $2")
                .bind(v)
                .bind(&card.id)
                .execute(&mut *tx)
                .await
                .map_err(|e| ToolError::Other(e.to_string()))?;
        }
    }
    audit::append_event(
        &ctx.pool,
        AppendEventParams {
            action: &format!("card.{kind}"),
            actor_user_id: Some(&ctx.user.id),
            session: None,
            request_id: Some(&ctx.request_id),
            workflow_id: None,
            node_id: None,
            tool: Some(&format!("cards.{kind}")),
            parameters_masked: Some(json!({
                "card_id": card.id,
                "kind": kind,
            })),
            risk_tier: params.get("tier").and_then(|v| v.as_str()),
            policy_reason: None,
            confirmation_status: None,
            mfa_status: None,
            result: "ok",
            error_code: None,
        },
    )
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    tx.commit()
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    Ok(json!({
        "card_id": card.id,
        "status": new_status,
    }))
}

pub async fn t_cards_issue(
    ctx: &ToolContext,
    params: &Value,
) -> Result<Value, ToolError> {
    let display_name = params
        .get("display_name")
        .and_then(|v| v.as_str())
        .unwrap_or("Virtual card");
    let single_tx = params
        .get("single_tx_limit_fen")
        .and_then(|v| v.as_i64())
        .unwrap_or(50_000);
    let monthly = params
        .get("monthly_limit_fen")
        .and_then(|v| v.as_i64())
        .unwrap_or(1_000_000);
    let card = crate::models::Card {
        id: ids::new_id("card"),
        user_id: ctx.user.id.clone(),
        display_name: display_name.to_string(),
        masked_number: format!("****{}", rand4()),
        expiry_month: 12,
        expiry_year: 2030,
        status: "active".to_string(),
        single_tx_limit_fen: Some(single_tx),
        monthly_limit_fen: Some(monthly),
        created_at: now(),
    };
    let mut tx = ctx
        .pool
        .begin()
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    sqlx::query(
        "INSERT INTO cards (id, user_id, display_name, masked_number, expiry_month, expiry_year, status, single_tx_limit_fen, monthly_limit_fen, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(&card.id)
    .bind(&card.user_id)
    .bind(&card.display_name)
    .bind(&card.masked_number)
    .bind(card.expiry_month)
    .bind(card.expiry_year)
    .bind(&card.status)
    .bind(card.single_tx_limit_fen)
    .bind(card.monthly_limit_fen)
    .bind(card.created_at)
    .execute(&mut *tx)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    audit::append_event(
        &ctx.pool,
        AppendEventParams {
            action: "card.issue",
            actor_user_id: Some(&ctx.user.id),
            session: None,
            request_id: Some(&ctx.request_id),
            workflow_id: None,
            node_id: None,
            tool: Some("cards.issue"),
            parameters_masked: Some(json!({
                "card_id": card.id,
                "display_name": card.display_name,
            })),
            risk_tier: params.get("tier").and_then(|v| v.as_str()),
            policy_reason: None,
            confirmation_status: None,
            mfa_status: None,
            result: "ok",
            error_code: None,
        },
    )
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    tx.commit()
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    Ok(json!({
        "card_id": card.id,
        "status": card.status,
        "masked_number": card.masked_number,
    }))
}

pub async fn t_wealth_subscribe(
    ctx: &ToolContext,
    params: &Value,
) -> Result<Value, ToolError> {
    let product_id = params
        .get("product_id")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("product_id required"))?;
    let amount_fen = params
        .get("amount_fen")
        .and_then(|v| v.as_i64())
        .ok_or(ToolError::Schema("amount_fen required"))?;
    let account_id = params
        .get("account_id")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("account_id required"))?;

    let mut tx = ctx
        .pool
        .begin()
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    let product: Option<crate::models::WealthProduct> =
        sqlx::query_as("SELECT * FROM wealth_products WHERE id = $1")
            .bind(product_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| ToolError::Other(e.to_string()))?;
    let product = product.ok_or(ToolError::NotFound("Product not found"))?;
    if amount_fen < product.min_subscription_fen {
        return Err(ToolError::BelowMinimum("Amount below product minimum"));
    }
    let mut account = lock_account(&mut *tx, account_id).await?;
    if account.user_id != ctx.user.id {
        return Err(ToolError::NotFound("Account not found"));
    }
    if account.available_balance_fen < amount_fen {
        return Err(ToolError::InsufficientFunds("Not enough available balance"));
    }
    let holding = crate::models::WealthHolding {
        id: ids::new_id("hold"),
        user_id: ctx.user.id.clone(),
        product_id: product.id.clone(),
        amount_fen,
        subscribed_at: now(),
        status: "active".to_string(),
        created_at: now(),
    };
    sqlx::query(
        "INSERT INTO wealth_holdings (id, user_id, product_id, amount_fen, subscribed_at, status, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(&holding.id)
    .bind(&holding.user_id)
    .bind(&holding.product_id)
    .bind(holding.amount_fen)
    .bind(holding.subscribed_at)
    .bind(&holding.status)
    .bind(holding.created_at)
    .execute(&mut *tx)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    let tx_row = crate::models::Transaction {
        id: ids::new_id("tx"),
        account_id: account.id.clone(),
        amount_fen: -amount_fen,
        direction: "debit".to_string(),
        category: "wealth".to_string(),
        merchant: product.name.clone(),
        memo: "".to_string(),
        occurred_at: now(),
        status: "posted".to_string(),
    };
    sqlx::query(
        "INSERT INTO transactions (id, account_id, amount_fen, direction, category, merchant, memo, occurred_at, status)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(&tx_row.id)
    .bind(&tx_row.account_id)
    .bind(tx_row.amount_fen)
    .bind(&tx_row.direction)
    .bind(&tx_row.category)
    .bind(&tx_row.merchant)
    .bind(&tx_row.memo)
    .bind(tx_row.occurred_at)
    .bind(&tx_row.status)
    .execute(&mut *tx)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    sqlx::query("UPDATE accounts SET available_balance_fen = available_balance_fen - $1 WHERE id = $2")
        .bind(amount_fen)
        .bind(&account.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    account.available_balance_fen -= amount_fen;
    audit::append_event(
        &ctx.pool,
        AppendEventParams {
            action: "wealth.subscribe",
            actor_user_id: Some(&ctx.user.id),
            session: None,
            request_id: Some(&ctx.request_id),
            workflow_id: None,
            node_id: None,
            tool: Some("wealth.subscribe"),
            parameters_masked: Some(json!({
                "product_id": product.id,
                "amount_fen": amount_fen,
            })),
            risk_tier: params.get("tier").and_then(|v| v.as_str()),
            policy_reason: None,
            confirmation_status: None,
            mfa_status: None,
            result: "ok",
            error_code: None,
        },
    )
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    tx.commit()
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    Ok(json!({
        "holding_id": holding.id,
        "product_id": product.id,
        "amount_fen": amount_fen,
    }))
}

pub async fn t_wealth_redeem(
    ctx: &ToolContext,
    params: &Value,
) -> Result<Value, ToolError> {
    let holding_id = params
        .get("holding_id")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("holding_id required"))?;
    let account_id = params
        .get("account_id")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("account_id required"))?;
    let mut tx = ctx
        .pool
        .begin()
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    let holding: Option<crate::models::WealthHolding> =
        sqlx::query_as("SELECT * FROM wealth_holdings WHERE id = $1")
            .bind(holding_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| ToolError::Other(e.to_string()))?;
    let holding = holding.ok_or(ToolError::NotFound("Holding not found"))?;
    if holding.user_id != ctx.user.id {
        return Err(ToolError::NotFound("Holding not found"));
    }
    if holding.status != "active" {
        tx.commit()
            .await
            .map_err(|e| ToolError::Other(e.to_string()))?;
        return Ok(json!({ "holding_id": holding.id, "status": holding.status }));
    }
    let mut account = lock_account(&mut *tx, account_id).await?;
    if account.user_id != ctx.user.id {
        return Err(ToolError::NotFound("Account not found"));
    }
    sqlx::query("UPDATE wealth_holdings SET status = 'redeemed' WHERE id = $1")
        .bind(&holding.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    let credit = crate::models::Transaction {
        id: ids::new_id("tx"),
        account_id: account.id.clone(),
        amount_fen: holding.amount_fen,
        direction: "credit".to_string(),
        category: "wealth".to_string(),
        merchant: "wealth-redeem".to_string(),
        memo: "".to_string(),
        occurred_at: now(),
        status: "posted".to_string(),
    };
    sqlx::query(
        "INSERT INTO transactions (id, account_id, amount_fen, direction, category, merchant, memo, occurred_at, status)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(&credit.id)
    .bind(&credit.account_id)
    .bind(credit.amount_fen)
    .bind(&credit.direction)
    .bind(&credit.category)
    .bind(&credit.merchant)
    .bind(&credit.memo)
    .bind(credit.occurred_at)
    .bind(&credit.status)
    .execute(&mut *tx)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    sqlx::query("UPDATE accounts SET available_balance_fen = available_balance_fen + $1 WHERE id = $2")
        .bind(holding.amount_fen)
        .bind(&account.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    account.available_balance_fen += holding.amount_fen;
    audit::append_event(
        &ctx.pool,
        AppendEventParams {
            action: "wealth.redeem",
            actor_user_id: Some(&ctx.user.id),
            session: None,
            request_id: Some(&ctx.request_id),
            workflow_id: None,
            node_id: None,
            tool: Some("wealth.redeem"),
            parameters_masked: Some(json!({
                "holding_id": holding.id,
                "amount_fen": holding.amount_fen,
            })),
            risk_tier: params.get("tier").and_then(|v| v.as_str()),
            policy_reason: None,
            confirmation_status: None,
            mfa_status: None,
            result: "ok",
            error_code: None,
        },
    )
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    tx.commit()
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    Ok(json!({
        "holding_id": holding.id,
        "status": "redeemed",
        "amount_fen": holding.amount_fen,
    }))
}

// --- Budgets --------------------------------------------------------------
//
// `t_budget_set` creates or updates a per-category monthly cap. If `limit_fen`
// is 0 or the budget already exists for that (user, category, account)
// triple, the existing row is updated. The agent and the optimization
// report use this to keep the dashboard in sync with the user's intent.
//
// `t_budget_list` returns the active budgets for a user with running usage
// (spend in the period vs. limit) so the chat agent can say "you've used
// 70% of your food budget this month" without a separate query.
//
// `t_budget_reserve` is the cross-scenario orchestration primitive: the
// chat can ask the agent to "reserve ¥1000 from my checking for the
// spouse's birthday in October" and the tool does it atomically — creates
// a savings goal, holds the cash, and records a budget_reservation so the
// optimizer / forecaster both see the commitment.

pub async fn t_budget_set(
    ctx: &ToolContext,
    params: &Value,
) -> Result<Value, ToolError> {
    let category = params
        .get("category")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("category required"))?
        .to_string();
    let limit_fen = params
        .get("limit_fen")
        .and_then(|v| v.as_i64())
        .ok_or(ToolError::Schema("limit_fen required"))?;
    if limit_fen <= 0 {
        return Err(ToolError::Schema("limit_fen must be > 0"));
    }
    let account_id = params
        .get("account_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let note = params
        .get("note")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Look up an existing budget for this (user, category, account) — if
    // present, UPDATE; otherwise INSERT. Keeps the API idempotent so the
    // agent can safely re-issue after a clarification.
    let existing: Option<Budget> = sqlx::query_as(
        "SELECT * FROM budgets WHERE user_id = $1 AND category = $2 \
         AND ((account_id IS NULL AND $3::text IS NULL) OR account_id = $3) \
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(&ctx.user.id)
    .bind(&category)
    .bind(account_id.as_deref())
    .fetch_optional(&ctx.pool)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;

    let budget = if let Some(mut b) = existing {
        sqlx::query(
            "UPDATE budgets SET limit_fen = $2, note = $3, updated_at = NOW() WHERE id = $1",
        )
        .bind(&b.id)
        .bind(limit_fen)
        .bind(note.as_deref())
        .execute(&ctx.pool)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
        b.limit_fen = limit_fen;
        b.note = note.clone();
        b
    } else {
        let id = ids::new_id("bgt");
        sqlx::query(
            "INSERT INTO budgets (id, user_id, category, limit_fen, account_id, note, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())",
        )
        .bind(&id)
        .bind(&ctx.user.id)
        .bind(&category)
        .bind(limit_fen)
        .bind(account_id.as_deref())
        .bind(note.as_deref())
        .execute(&ctx.pool)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
        Budget {
            id,
            user_id: ctx.user.id.clone(),
            category,
            limit_fen,
            period_start: None,
            period_end: None,
            account_id,
            note,
            created_at: now(),
            updated_at: now(),
        }
    };

    audit::append_event(
        &ctx.pool,
        AppendEventParams {
            action: "budget.set",
            actor_user_id: Some(&ctx.user.id),
            session: None,
            request_id: Some(&ctx.request_id),
            workflow_id: None,
            node_id: None,
            tool: Some("budgets.set"),
            parameters_masked: Some(json!({
                "category": budget.category,
                "limit_fen": budget.limit_fen,
                "account_id": budget.account_id,
            })),
            risk_tier: params.get("tier").and_then(|v| v.as_str()),
            policy_reason: None,
            confirmation_status: None,
            mfa_status: None,
            result: "ok",
            error_code: None,
        },
    )
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;

    Ok(json!({
        "budget_id": budget.id,
        "category": budget.category,
        "limit_fen": budget.limit_fen,
        "account_id": budget.account_id,
        "note": budget.note,
    }))
}

pub async fn t_budget_list(
    ctx: &ToolContext,
    _params: &Value,
) -> Result<Value, ToolError> {
    let budgets: Vec<Budget> = sqlx::query_as(
        "SELECT * FROM budgets WHERE user_id = $1 ORDER BY category ASC, created_at DESC",
    )
    .bind(&ctx.user.id)
    .fetch_all(&ctx.pool)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;

    // For each budget, compute usage over the last 30 days (the same window
    // /reports/spending uses). Per-account budgets only count debits on
    // that account; global ones count everything.
    let mut out: Vec<serde_json::Value> = Vec::with_capacity(budgets.len());
    for b in &budgets {
        let spend: i64 = if let Some(acc) = &b.account_id {
            sqlx::query_scalar(
                "SELECT COALESCE(SUM(-t.amount_fen), 0)::BIGINT FROM transactions t \
                 JOIN accounts a ON a.id = t.account_id \
                 WHERE a.user_id = $1 AND a.id = $2 AND t.category = $3 \
                 AND t.amount_fen < 0 AND t.occurred_at >= NOW() - INTERVAL '30 days'",
            )
            .bind(&ctx.user.id)
            .bind(acc)
            .bind(&b.category)
            .fetch_one(&ctx.pool)
            .await
            .map_err(|e| ToolError::Other(e.to_string()))?
        } else {
            sqlx::query_scalar(
                "SELECT COALESCE(SUM(-t.amount_fen), 0)::BIGINT FROM transactions t \
                 JOIN accounts a ON a.id = t.account_id \
                 WHERE a.user_id = $1 AND t.category = $2 \
                 AND t.amount_fen < 0 AND t.occurred_at >= NOW() - INTERVAL '30 days'",
            )
            .bind(&ctx.user.id)
            .bind(&b.category)
            .fetch_one(&ctx.pool)
            .await
            .map_err(|e| ToolError::Other(e.to_string()))?
        };
        let pct = if b.limit_fen > 0 {
            (spend as f64 / b.limit_fen as f64) * 100.0
        } else {
            0.0
        };
        out.push(json!({
            "id": b.id,
            "category": b.category,
            "limit_fen": b.limit_fen,
            "spent_fen": spend,
            "remaining_fen": (b.limit_fen - spend).max(0),
            "utilization_pct": pct,
            "breached": spend > b.limit_fen,
            "account_id": b.account_id,
            "note": b.note,
        }));
    }

    Ok(json!({ "budgets": out }))
}

pub async fn t_budget_reserve(
    ctx: &ToolContext,
    params: &Value,
) -> Result<Value, ToolError> {
    let label = params
        .get("label")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("label required"))?
        .to_string();
    let amount_fen = params
        .get("amount_fen")
        .and_then(|v| v.as_i64())
        .ok_or(ToolError::Schema("amount_fen required"))?;
    if amount_fen <= 0 {
        return Err(ToolError::Schema("amount_fen must be > 0"));
    }
    let account_id = params
        .get("account_id")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("account_id required"))?;
    let due_at = params
        .get("due_at")
        .and_then(|v| v.as_str())
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&Utc));
    let budget_id = params
        .get("budget_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let mut tx = ctx
        .pool
        .begin()
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    let mut account = lock_account(&mut *tx, account_id).await?;
    if account.user_id != ctx.user.id {
        return Err(ToolError::NotFound("Account not found"));
    }
    if account.available_balance_fen < amount_fen {
        return Err(ToolError::InsufficientFunds(
            "Not enough available balance to reserve",
        ));
    }

    // Move money from available → reserved. We back the hold with a
    // savings goal so the user sees it on the goals page and the existing
    // reservation/release machinery can settle it later.
    let goal_id = ids::new_id("goal");
    let goal_name = format!("Reserved: {}", label);
    sqlx::query(
        "INSERT INTO savings_goals (id, user_id, name, target_fen, created_at) VALUES ($1, $2, $3, $4, NOW())",
    )
    .bind(&goal_id)
    .bind(&ctx.user.id)
    .bind(&goal_name)
    .bind(amount_fen)
    .execute(&mut *tx)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;

    let res_id = ids::new_id("bres");
    sqlx::query(
        "INSERT INTO reservations (id, goal_id, amount_fen, state, source_account_id, created_at) VALUES ($1, $2, $3, 'held', $4, NOW())",
    )
    .bind(&res_id)
    .bind(&goal_id)
    .bind(amount_fen)
    .bind(&account.id)
    .execute(&mut *tx)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;

    sqlx::query(
        "INSERT INTO budget_reservations (id, user_id, budget_id, goal_id, label, amount_fen, due_at, source_account_id, state, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'held', NOW())",
    )
    .bind(&res_id)
    .bind(&ctx.user.id)
    .bind(budget_id.as_deref())
    .bind(&goal_id)
    .bind(&label)
    .bind(amount_fen)
    .bind(due_at)
    .bind(&account.id)
    .execute(&mut *tx)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;

    sqlx::query(
        "UPDATE accounts SET available_balance_fen = available_balance_fen - $1, reserved_balance_fen = reserved_balance_fen + $2 WHERE id = $3",
    )
    .bind(amount_fen)
    .bind(amount_fen)
    .bind(&account.id)
    .execute(&mut *tx)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    account.available_balance_fen -= amount_fen;
    account.reserved_balance_fen += amount_fen;

    // Set a reminder one day before due_at (if set) so the agent can surface
    // "spouse birthday tomorrow — your reservation is ready" without an
    // extra tool call.
    if let Some(due) = due_at {
        let remind_at = due - chrono::Duration::days(1);
        if remind_at > now() {
            let rem_id = ids::new_id("rem");
            let _ = sqlx::query(
                "INSERT INTO reminders (id, user_id, kind, subject, due_at, status, linked_subscription_id, created_at) VALUES ($1, $2, 'budget_reservation_due', $3, $4, 'scheduled', NULL, NOW())",
            )
            .bind(&rem_id)
            .bind(&ctx.user.id)
            .bind(format!("{} due in 1 day — reservation ready", label))
            .bind(remind_at)
            .execute(&mut *tx)
            .await;
        }
    }

    audit::append_event(
        &ctx.pool,
        AppendEventParams {
            action: "budget.reserve",
            actor_user_id: Some(&ctx.user.id),
            session: None,
            request_id: Some(&ctx.request_id),
            workflow_id: None,
            node_id: None,
            tool: Some("budgets.reserve"),
            parameters_masked: Some(json!({
                "label": label,
                "amount_fen": amount_fen,
                "due_at": due_at,
                "account_id": account.id,
            })),
            risk_tier: params.get("tier").and_then(|v| v.as_str()),
            policy_reason: None,
            confirmation_status: None,
            mfa_status: None,
            result: "ok",
            error_code: None,
        },
    )
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;

    tx.commit()
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;

    Ok(json!({
        "reservation_id": res_id,
        "goal_id": goal_id,
        "label": label,
        "amount_fen": amount_fen,
        "account_id": account.id,
        "due_at": due_at,
        "state": "held",
    }))
}

// --- Registry ------------------------------------------------------------

pub type ToolFn = for<'a> fn(
    &'a ToolContext,
    &'a Value,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value, ToolError>> + Send + 'a>>;

pub fn registry() -> std::collections::HashMap<&'static str, ToolFn> {
    let mut m: std::collections::HashMap<&'static str, ToolFn> = std::collections::HashMap::new();
    m.insert("accounts.balance_check", |ctx, p| Box::pin(t_accounts_balance_check(ctx, p)));
    m.insert("profile.resolve_date", |ctx, p| Box::pin(t_profile_resolve_date(ctx, p)));
    m.insert("transfer.create_pending", |ctx, p| Box::pin(t_transfer_create_pending(ctx, p)));
    m.insert("subscriptions.cancel", |ctx, p| Box::pin(t_subscriptions_cancel(ctx, p)));
    m.insert("subscriptions.remind", |ctx, p| Box::pin(t_subscriptions_remind(ctx, p)));
    m.insert("goals.create_reservation", |ctx, p| Box::pin(t_goals_create_reservation(ctx, p)));
    m.insert("goals.consume_reservation", |ctx, p| Box::pin(t_goals_consume_reservation(ctx, p)));
    m.insert("goals.release_reservation", |ctx, p| Box::pin(t_goals_release_reservation(ctx, p)));
    m.insert("reminders.create", |ctx, p| Box::pin(t_reminders_create(ctx, p)));
    m.insert(
        "connectors.merchant.order",
        |ctx, p| Box::pin(t_connectors_merchant_order(ctx, p)),
    );
    m.insert("cards.issue", |ctx, p| Box::pin(t_cards_issue(ctx, p)));
    m.insert("cards.action", |ctx, p| Box::pin(t_cards_action(ctx, p)));
    m.insert("wealth.subscribe", |ctx, p| Box::pin(t_wealth_subscribe(ctx, p)));
    m.insert("wealth.redeem", |ctx, p| Box::pin(t_wealth_redeem(ctx, p)));
    // Read-only agent helpers (full data, no state change).
    m.insert("accounts.list", |ctx, p| Box::pin(t_list_accounts(ctx, p)));
    m.insert("payees.list", |ctx, p| Box::pin(t_list_payees(ctx, p)));
    m.insert("subscriptions.list", |ctx, p| Box::pin(t_list_subscriptions(ctx, p)));
    m.insert("cards.list", |ctx, p| Box::pin(t_list_cards(ctx, p)));
    m.insert("accounts.balance_summary", |ctx, p| Box::pin(t_balance_summary(ctx, p)));
    m.insert("spending.report", |ctx, p| Box::pin(t_spending_report(ctx, p)));
    m.insert("transactions.recent", |ctx, p| Box::pin(t_recent_transactions(ctx, p)));
    m.insert("wealth.summary", |ctx, p| Box::pin(t_wealth_summary(ctx, p)));
    m.insert("user.profile", |ctx, p| Box::pin(t_user_profile(ctx, p)));
    // Phase 6 — smart-transfer agent tools
    m.insert("smart.payees.suggest", |ctx, p| Box::pin(t_smart_payees_suggest(ctx, p)));
    m.insert("smart.payee.resolve", |ctx, p| Box::pin(t_smart_payee_resolve(ctx, p)));
    m.insert("smart.transfer.parse", |ctx, p| Box::pin(t_smart_transfer_parse(ctx, p)));
    m.insert(
        "smart.transfer.security_review",
        |ctx, p| Box::pin(t_smart_transfer_security_review(ctx, p)),
    );
    m.insert("split_bill.create", |ctx, p| Box::pin(t_split_bill_create(ctx, p)));
    m.insert("split_bill.list", |ctx, p| Box::pin(t_split_bill_list(ctx, p)));
    // Phase 5 demo features
    m.insert("forecast.simulate", |ctx, p| Box::pin(t_forecast_simulate(ctx, p)));
    m.insert("fraud.score", |ctx, p| Box::pin(t_fraud_score(ctx, p)));
    m.insert("redteam.test", |ctx, p| Box::pin(t_redteam_test(ctx, p)));
    m.insert("voice.tts", |ctx, p| Box::pin(t_voice_tts(ctx, p)));
    // Budgets: caps + cross-scenario reservations.
    m.insert("budgets.set", |ctx, p| Box::pin(t_budget_set(ctx, p)));
    m.insert("budgets.list", |ctx, p| Box::pin(t_budget_list(ctx, p)));
    m.insert("budgets.reserve", |ctx, p| Box::pin(t_budget_reserve(ctx, p)));
    m
}

// --- Transaction-scoped helpers ----------------------------------------
//
// `tx` is `&mut *transaction` to a sqlx `Transaction<sqlx::Postgres>`. We
// accept any executor so the same helpers can be called from pool-level
// queries if needed.

/// Lock an account row for update inside an open transaction.
pub async fn lock_account<'c, E>(
    executor: E,
    account_id: &str,
) -> Result<Account, sqlx::Error>
where
    E: sqlx::Executor<'c, Database = sqlx::Postgres>,
{
    sqlx::query_as::<_, Account>(
        "SELECT * FROM accounts WHERE id = $1 FOR UPDATE",
    )
    .bind(account_id)
    .fetch_one(executor)
    .await
}

/// Resolve a payee by id and ensure it belongs to the given owner.
pub async fn resolve_payee<'c, E>(
    executor: E,
    owner_user_id: &str,
    payee_id: &str,
) -> Result<Payee, sqlx::Error>
where
    E: sqlx::Executor<'c, Database = sqlx::Postgres>,
{
    let p: Payee = sqlx::query_as::<_, Payee>("SELECT * FROM payees WHERE id = ?")
        .bind(payee_id)
        .fetch_one(executor)
        .await?;
    if p.owner_user_id != owner_user_id {
        return Err(sqlx::Error::RowNotFound);
    }
    Ok(p)
}

// --- Read-only agent tools ----------------------------------------------
//
// The agent has a full data summary in its system prompt already, but
// these tools let the model call back into the backend for fresh data
// when the user asks a question that needs a specific slice.

pub async fn t_list_accounts(
    ctx: &ToolContext,
    _args: &Value,
) -> Result<Value, ToolError> {
    let rows: Vec<Account> = sqlx::query_as(
        "SELECT * FROM accounts WHERE user_id = $1 ORDER BY created_at ASC",
    )
    .bind(&ctx.user.id)
    .fetch_all(&ctx.pool)
    .await
    ?;
    let out: Vec<Value> = rows
        .iter()
        .map(|a| {
            json!({
                "id": a.id,
                "display_name": a.display_name,
                "masked_number": a.masked_number,
                "currency": a.currency,
                "available_balance_fen": a.available_balance_fen,
                "reserved_balance_fen": a.reserved_balance_fen,
            })
        })
        .collect();
    Ok(json!({ "accounts": out, "count": out.len() }))
}

pub async fn t_list_payees(
    ctx: &ToolContext,
    _args: &Value,
) -> Result<Value, ToolError> {
    let rows: Vec<Payee> = sqlx::query_as("SELECT * FROM payees WHERE owner_user_id = $1")
        .bind(&ctx.user.id)
        .fetch_all(&ctx.pool)
        .await
        ?;
    let out: Vec<Value> = rows
        .iter()
        .map(|p| {
            json!({
                "id": p.id,
                "display_name": p.display_name,
                "phone_tail": p.phone_tail,
                "verified": p.verified,
            })
        })
        .collect();
    Ok(json!({ "payees": out, "count": out.len() }))
}

pub async fn t_list_subscriptions(
    ctx: &ToolContext,
    args: &Value,
) -> Result<Value, ToolError> {
    let status_filter = args.get("status").and_then(|v| v.as_str()).unwrap_or("active");
    let rows: Vec<Subscription> = sqlx::query_as(
        "SELECT id, user_id, merchant, estimated_amount_fen, interval_days, next_expected_at, last_charge_at, evidence::text AS evidence, cancellation_status, cancelled_at, created_at FROM subscriptions WHERE user_id = $1 AND ($2 = 'all' OR cancellation_status = $2) ORDER BY merchant ASC",
    )
    .bind(&ctx.user.id)
    .bind(status_filter)
    .fetch_all(&ctx.pool)
    .await
    ?;
    let monthly_total: i64 = rows
        .iter()
        .filter(|s| s.cancellation_status == "active")
        .map(|s| (s.estimated_amount_fen as f64 * 30.0 / s.interval_days.max(1) as f64) as i64)
        .sum();
    let out: Vec<Value> = rows
        .iter()
        .map(|s| {
            json!({
                "id": s.id,
                "merchant": s.merchant,
                "estimated_amount_fen": s.estimated_amount_fen,
                "interval_days": s.interval_days,
                "next_expected_at": s.next_expected_at,
                "last_charge_at": s.last_charge_at,
                "cancellation_status": s.cancellation_status,
            })
        })
        .collect();
    Ok(json!({
        "subscriptions": out,
        "count": out.len(),
        "active_count": rows.iter().filter(|s| s.cancellation_status == "active").count(),
        "monthly_total_fen": monthly_total,
    }))
}

pub async fn t_list_cards(
    ctx: &ToolContext,
    _args: &Value,
) -> Result<Value, ToolError> {
    let rows: Vec<Card> = sqlx::query_as("SELECT * FROM cards WHERE user_id = $1")
        .bind(&ctx.user.id)
        .fetch_all(&ctx.pool)
        .await
        ?;
    let out: Vec<Value> = rows
        .iter()
        .map(|c| {
            json!({
                "id": c.id,
                "display_name": c.display_name,
                "masked_number": c.masked_number,
                "status": c.status,
                "single_tx_limit_fen": c.single_tx_limit_fen,
                "monthly_limit_fen": c.monthly_limit_fen,
            })
        })
        .collect();
    Ok(json!({ "cards": out, "count": out.len() }))
}

pub async fn t_balance_summary(
    ctx: &ToolContext,
    _args: &Value,
) -> Result<Value, ToolError> {
    let rows: Vec<Account> = sqlx::query_as(
        "SELECT * FROM accounts WHERE user_id = $1 ORDER BY created_at ASC",
    )
    .bind(&ctx.user.id)
    .fetch_all(&ctx.pool)
    .await
    ?;
    let mut available: i64 = 0;
    let mut reserved: i64 = 0;
    for a in &rows {
        available += a.available_balance_fen;
        reserved += a.reserved_balance_fen;
    }
    Ok(json!({
        "available_balance_fen": available,
        "reserved_balance_fen": reserved,
        "total_fen": available + reserved,
        "currency": rows.first().map(|a| a.currency.clone()).unwrap_or_else(|| "CNY".to_string()),
        "account_count": rows.len(),
    }))
}

pub async fn t_spending_report(
    ctx: &ToolContext,
    args: &Value,
) -> Result<Value, ToolError> {
    let days = args.get("days").and_then(|v| v.as_i64()).unwrap_or(30);
    let rows: Vec<Transaction> = sqlx::query_as(
        "SELECT t.* FROM transactions t JOIN accounts a ON a.id = t.account_id \
         WHERE a.user_id = $1 AND t.amount_fen < 0 \
         AND t.occurred_at >= NOW() - ($2::int * INTERVAL '1 day') \
         ORDER BY t.occurred_at DESC",
    )
    .bind(&ctx.user.id)
    .bind(days as i32)
    .fetch_all(&ctx.pool)
    .await
    ?;
    let mut by_cat: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();
    let mut total: i64 = 0;
    for t in &rows {
        let amt = -t.amount_fen;
        total += amt;
        *by_cat.entry(t.category.clone()).or_insert(0) += amt;
    }
    let mut cats: Vec<(String, i64)> = by_cat.into_iter().collect();
    cats.sort_by(|a, b| b.1.cmp(&a.1));
    let cats_json: Vec<Value> = cats
        .iter()
        .map(|(c, amt)| json!({ "category": c, "amount_fen": amt }))
        .collect();
    Ok(json!({
        "days": days,
        "total_spend_fen": total,
        "transaction_count": rows.len(),
        "categories": cats_json,
    }))
}

pub async fn t_recent_transactions(
    ctx: &ToolContext,
    args: &Value,
) -> Result<Value, ToolError> {
    let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(20).min(100) as i64;
    let rows: Vec<Transaction> = sqlx::query_as(
        "SELECT t.* FROM transactions t JOIN accounts a ON a.id = t.account_id \
         WHERE a.user_id = $1 ORDER BY t.occurred_at DESC LIMIT $2",
    )
    .bind(&ctx.user.id)
    .bind(limit)
    .fetch_all(&ctx.pool)
    .await
    ?;
    let out: Vec<Value> = rows
        .iter()
        .map(|t| {
            json!({
                "id": t.id,
                "amount_fen": t.amount_fen,
                "direction": t.direction,
                "category": t.category,
                "merchant": t.merchant,
                "memo": t.memo,
                "occurred_at": t.occurred_at,
            })
        })
        .collect();
    Ok(json!({ "transactions": out, "count": out.len() }))
}

pub async fn t_wealth_summary(
    ctx: &ToolContext,
    _args: &Value,
) -> Result<Value, ToolError> {
    let holdings: Vec<WealthHolding> =
        sqlx::query_as("SELECT * FROM wealth_holdings WHERE user_id = $1")
            .bind(&ctx.user.id)
            .fetch_all(&ctx.pool)
            .await
            ?;
    let products: Vec<WealthProduct> = sqlx::query_as("SELECT * FROM wealth_products")
        .fetch_all(&ctx.pool)
        .await
        ?;
    let product_map: std::collections::HashMap<String, &WealthProduct> = products
        .iter()
        .map(|p| (p.id.clone(), p))
        .collect();
    let mut total: i64 = 0;
    let rows: Vec<Value> = holdings
        .iter()
        .map(|h| {
            total += h.amount_fen;
            let product = product_map.get(&h.product_id);
            json!({
                "id": h.id,
                "amount_fen": h.amount_fen,
                "status": h.status,
                "product": product.map(|p| json!({
                    "id": p.id,
                    "name": p.name,
                    "category": p.category,
                    "risk_level": p.risk_level,
                })),
            })
        })
        .collect();
    Ok(json!({
        "holdings": rows,
        "total_fen": total,
        "count": rows.len(),
    }))
}

// --- Smart-transfer agent tools -------------------------------------------
//
// These wrap `crate::smart_transfers` so the planner can invoke them
// from a chat turn. They never mutate state; the mutating parts
// (transfer, payee, split-bill) still go through the existing
// tool registry's write tools.

pub async fn t_smart_payees_suggest(
    ctx: &ToolContext,
    args: &Value,
) -> Result<Value, ToolError> {
    let q = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("query is required"))?;
    let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(5);
    let out = crate::smart_transfers::suggest_payees(&ctx.pool, &ctx.user, q, limit)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    serde_json::to_value(&out).map_err(|e| ToolError::Other(e.to_string()))
}

pub async fn t_smart_payee_resolve(
    ctx: &ToolContext,
    args: &Value,
) -> Result<Value, ToolError> {
    let q = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("query is required"))?;
    let out = crate::smart_transfers::resolve(&ctx.pool, &ctx.user, q)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    serde_json::to_value(&out).map_err(|e| ToolError::Other(e.to_string()))
}

pub async fn t_smart_transfer_parse(
    ctx: &ToolContext,
    args: &Value,
) -> Result<Value, ToolError> {
    let text = args
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("text is required"))?;
    let settings = crate::config::get_settings();
    let out = crate::smart_transfers::parse_request(&ctx.pool, settings, &ctx.user, text).await;
    Ok(out)
}

pub async fn t_smart_transfer_security_review(
    ctx: &ToolContext,
    args: &Value,
) -> Result<Value, ToolError> {
    use crate::models::Payee;
    let payee: Payee = if let Some(pid) = args.get("payee_id").and_then(|v| v.as_str()) {
        sqlx::query_as::<_, Payee>("SELECT * FROM payees WHERE id = $1 AND owner_user_id = $2")
            .bind(pid)
            .bind(&ctx.user.id)
            .fetch_optional(&ctx.pool)
            .await
            .map_err(|e| ToolError::Other(e.to_string()))?
            .ok_or(ToolError::NotFound("Payee not found"))?
    } else if let Some(name) = args.get("payee_name").and_then(|v| v.as_str()) {
        sqlx::query_as::<_, Payee>(
            "SELECT * FROM payees WHERE owner_user_id = $1 AND LOWER(display_name) LIKE LOWER($2) ORDER BY created_at ASC LIMIT 1",
        )
        .bind(&ctx.user.id)
        .bind(format!("%{}%", name))
        .fetch_optional(&ctx.pool)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?
        .unwrap_or(Payee {
            id: "transient".into(),
            owner_user_id: ctx.user.id.clone(),
            display_name: name.into(),
            phone_tail: None,
            mock_destination: "transient".into(),
            verified: false,
            first_used_at: None,
            created_at: chrono::Utc::now(),
        })
    } else {
        return Err(ToolError::Schema("payee_id or payee_name is required"));
    };
    let amount_fen = args
        .get("amount_fen")
        .and_then(|v| v.as_i64())
        .ok_or(ToolError::Schema("amount_fen is required"))?;
    let memo = args.get("memo").and_then(|v| v.as_str()).unwrap_or("");
    let schedule = args
        .get("schedule_iso")
        .and_then(|v| v.as_str())
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&chrono::Utc));
    let settings = crate::config::get_settings();
    let review =
        crate::smart_transfers::security_review(&ctx.pool, settings, &ctx.user, &payee, amount_fen, memo, schedule)
            .await;
    serde_json::to_value(&review).map_err(|e| ToolError::Other(e.to_string()))
}

pub async fn t_split_bill_create(
    ctx: &ToolContext,
    args: &Value,
) -> Result<Value, ToolError> {
    let title = args
        .get("title")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("title is required"))?;
    let total_fen = args
        .get("total_fen")
        .and_then(|v| v.as_i64())
        .ok_or(ToolError::Schema("total_fen is required"))?;
    let memo = args.get("memo").and_then(|v| v.as_str()).unwrap_or("");
    let currency = args.get("currency").and_then(|v| v.as_str()).unwrap_or("CNY");
    let source_account_id = args.get("source_account_id").and_then(|v| v.as_str());
    let parts_in = args
        .get("parts")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mut parts: Vec<crate::smart_transfers::NewPart> = Vec::new();
    for p in parts_in {
        let share_fen = p
            .get("share_fen")
            .and_then(|v| v.as_i64())
            .ok_or(ToolError::Schema("parts[].share_fen is required"))?;
        let name = p
            .get("participant_name")
            .and_then(|v| v.as_str())
            .unwrap_or("Guest")
            .to_string();
        parts.push(crate::smart_transfers::NewPart {
            payee_id: p.get("payee_id").and_then(|v| v.as_str()).map(|s| s.to_string()),
            participant_name: name,
            participant_phone_tail: p
                .get("participant_phone_tail")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            share_fen,
        });
    }
    if parts.is_empty() {
        return Err(ToolError::Schema("parts[] must contain at least one share"));
    }
    let bill = crate::smart_transfers::create_split_bill(
        &ctx.pool,
        &ctx.user,
        title,
        memo,
        total_fen,
        currency,
        source_account_id,
        None,
        parts,
    )
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    serde_json::to_value(&bill).map_err(|e| ToolError::Other(e.to_string()))
}

pub async fn t_split_bill_list(
    ctx: &ToolContext,
    _args: &Value,
) -> Result<Value, ToolError> {
    let out = crate::smart_transfers::list_split_bills(&ctx.pool, &ctx.user)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    serde_json::to_value(&out).map_err(|e| ToolError::Other(e.to_string()))
}

pub async fn t_user_profile(
    ctx: &ToolContext,
    _args: &Value,
) -> Result<Value, ToolError> {
    Ok(json!({
        "id": ctx.user.id,
        "display_name": ctx.user.display_name,
        "preferred_language": ctx.user.preferred_language,
        "role": ctx.user.role,
        "risk_tolerance": ctx.user.risk_tolerance,
        "horizon": ctx.user.horizon,
        "safety_lock_state": ctx.user.safety_lock_state,
    }))
}

// --- Phase 5 demo tools ----------------------------------------------------
//
// These wrap the corresponding HTTP endpoints so the planner can call them
// during a chat turn. They never touch mutable state — every one returns
// "ok" data the LLM can narrate over.

pub async fn t_forecast_simulate(
    ctx: &ToolContext,
    args: &Value,
) -> Result<Value, ToolError> {
    use crate::config;
    let amount_cny = args
        .get("amount_cny")
        .and_then(|v| v.as_f64())
        .ok_or(ToolError::Schema("amount_cny is required"))?;
    if amount_cny <= 0.0 {
        return Err(ToolError::Schema("amount_cny must be positive"));
    }
    let spend_month_index = args
        .get("spend_month_index")
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;
    let months = args
        .get("months")
        .and_then(|v| v.as_i64())
        .unwrap_or(12) as i32;
    let amount_fen = (amount_cny * 100.0).round() as i64;
    let settings = config::get_settings();
    let out = crate::forecast::run(
        &ctx.pool,
        settings,
        &ctx.user,
        amount_fen,
        spend_month_index,
        months,
    )
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    serde_json::to_value(&out).map_err(|e| ToolError::Other(e.to_string()))
}

pub async fn t_fraud_score(
    ctx: &ToolContext,
    args: &Value,
) -> Result<Value, ToolError> {
    use crate::config;
    use crate::models::Payee;
    let amount_fen = args
        .get("amount_fen")
        .and_then(|v| v.as_i64())
        .ok_or(ToolError::Schema("amount_fen is required"))?;
    if amount_fen <= 0 {
        return Err(ToolError::Schema("amount_fen must be positive"));
    }
    let memo = args.get("memo").and_then(|v| v.as_str()).unwrap_or("");
    let payee: Payee = if let Some(pid) = args.get("payee_id").and_then(|v| v.as_str()) {
        sqlx::query_as::<_, Payee>("SELECT * FROM payees WHERE id = $1 AND owner_user_id = $2")
            .bind(pid)
            .bind(&ctx.user.id)
            .fetch_optional(&ctx.pool)
            .await
            .map_err(|e| ToolError::Other(e.to_string()))?
            .ok_or(ToolError::NotFound("Payee not found"))?
    } else if let Some(name) = args.get("payee_name").and_then(|v| v.as_str()) {
        sqlx::query_as::<_, Payee>(
            "SELECT * FROM payees WHERE owner_user_id = $1 AND LOWER(display_name) LIKE LOWER($2) ORDER BY created_at ASC LIMIT 1",
        )
        .bind(&ctx.user.id)
        .bind(format!("%{}%", name))
        .fetch_optional(&ctx.pool)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?
        .unwrap_or(Payee {
            id: "transient".to_string(),
            owner_user_id: ctx.user.id.clone(),
            display_name: name.to_string(),
            phone_tail: None,
            mock_destination: "transient".to_string(),
            verified: false,
            first_used_at: None,
            created_at: chrono::Utc::now(),
        })
    } else {
        return Err(ToolError::Schema("payee_id or payee_name is required"));
    };
    let mut assessment = crate::fraud::score_transfer(&ctx.pool, &ctx.user, &payee, amount_fen, memo)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    if !assessment.flags.is_empty() {
        if let Some(narrative) =
            crate::fraud::narrate_scam_flags(config::get_settings(), &assessment, &payee.display_name, amount_fen)
                .await
        {
            assessment.narrative = Some(narrative);
        }
    }
    let injection_phrase = crate::policy::detect_injection(memo);
    Ok(json!({
        "assessment": assessment,
        "payee": {
            "id": payee.id,
            "display_name": payee.display_name,
            "verified": payee.verified,
            "first_used_at": payee.first_used_at,
        },
        "policy_injection_phrase": injection_phrase,
    }))
}

pub async fn t_redteam_test(
    _ctx: &ToolContext,
    args: &Value,
) -> Result<Value, ToolError> {
    let text = args
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("text is required"))?;
    let where_ = args
        .get("where")
        .and_then(|v| v.as_str())
        .unwrap_or("memo");
    let phrase = crate::policy::detect_injection(text);
    let sanitized = crate::redteam::sanitize(text);
    let view = crate::redteam::llm_view(where_, &sanitized);
    Ok(json!({
        "where": where_,
        "blocked": phrase.is_some(),
        "phrase": phrase,
        "sanitized_length": sanitized.chars().count(),
        "sanitized": sanitized,
        "llm_view": view,
    }))
}

pub async fn t_voice_tts(
    _ctx: &ToolContext,
    args: &Value,
) -> Result<Value, ToolError> {
    let text = args
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or(ToolError::Schema("text is required"))?;
    let out = crate::voice::build_tts_out(text);
    serde_json::to_value(&out).map_err(|e| ToolError::Other(e.to_string()))
}

// --- Tool name aliases ---------------------------------------------------
//
// The LLM is free to use natural names; we map them to the canonical
// allowlist names before dispatch. Unknown names fall through unchanged
// so the planner's allowlist check still catches typos.

pub fn canonicalize_tool_name(name: &str) -> &str {
    match name {
        // Read-only
        "list_accounts" | "accounts.list" | "get_accounts" | "show_accounts" => "accounts.list",
        "list_payees" | "payees.list" | "get_payees" | "show_payees" => "payees.list",
        "list_subscriptions" | "subscriptions.list" | "get_subscriptions"
        | "show_subscriptions" | "my_subscriptions" | "subscription_list" => "subscriptions.list",
        "list_cards" | "cards.list" | "get_cards" | "show_cards" | "my_cards" => "cards.list",
        "list_wealth" | "wealth.list" | "wealth_holdings" | "wealth_summary" => "wealth.summary",
        "balance" | "get_balance" | "check_balance" | "balance_summary" | "account_balance"
        | "show_balance" => "accounts.balance_summary",
        "spending" | "spending_report" | "spend_report" | "report_spending"
        | "show_spending" => "spending.report",
        "recent" | "recent_transactions" | "transactions" | "my_transactions" => "transactions.recent",
        "profile" | "user_profile" | "me" | "who_am_i" => "user.profile",
        // Action aliases
        "cancel_subscription" | "cancel_my_subscription" | "unsubscribe" => "subscriptions.cancel",
        "remind_me" | "remind_subscription" | "set_reminder" => "subscriptions.remind",
        "create_reminder" | "add_reminder" | "new_reminder" => "reminders.create",
        "send_money" | "pay" | "transfer" | "make_transfer" | "send" | "wire"
        | "transfer_money" => "transfer.create_pending",
        "issue_card" | "new_card" | "create_card" => "cards.issue",
        "freeze_card" | "unfreeze_card" | "cancel_card" | "card_action" => "cards.action",
        "subscribe_wealth" | "buy_fund" | "invest" | "wealth_invest" => "wealth.subscribe",
        "redeem_wealth" | "sell_fund" | "wealth_redeem" => "wealth.redeem",
        "save_goal" | "reserve_goal" | "create_goal" | "new_goal" => "goals.create_reservation",
        "use_goal" | "consume_reservation" => "goals.consume_reservation",
        "release_goal" | "release_reservation" => "goals.release_reservation",
        // Phase 5 features
        "forecast" | "forecast_simulation" | "what_if" | "can_i_afford" => "forecast.simulate",
        "fraud_score" | "scam_check" | "check_scam" => "fraud.score",
        "redteam" | "injection_test" => "redteam.test",
        "speak" | "tts" | "say" => "voice.tts",
        // Budgets — both informal ("set a budget for food at 2000") and
        // canonical forms.
        "set_budget" | "create_budget" | "cap_spending" | "limit_spending"
        | "budget_set" | "budgets.set" | "budget" | "my_budget" => "budgets.set",
        "list_budgets" | "show_budgets" | "my_budgets" | "budgets.list"
        | "budget_status" => "budgets.list",
        "reserve_money" | "set_aside" | "put_aside" | "save_for"
        | "budgets.reserve" | "reserve_budget" => "budgets.reserve",
        // Pass-through
        other => other,
    }
}
