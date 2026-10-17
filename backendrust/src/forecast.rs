//! "Can I afford it?" what-if forecaster.
//!
//! Given a scenario ("spend X fen in Y months"), project the user's
//! balance forward by computing an expected monthly cashflow from the
//! ledger (rolling 90-day averages) and the known upcoming bills /
//! subscriptions. The numbers come from the database — the LLM only
//! narrates the result. This separation is the same non-negotiable
//! the rest of BankPilot uses: code computes, the model explains.
//!
//! Output shape:
//!   {
//!     months: 12,
//!     starting_balance_fen: ...,
//!     avg_income_fen: ...,
//!     avg_expense_fen: ...,
//!     monthly_subscriptions_fen: ...,
//!     upcoming_bills_fen: [...],     // [{subject, due_at, amount_fen}]
//!     series: [{month_index, label, projected_balance_fen, baseline_fen}],
//!     scenario: {amount_fen, spend_month_index},
//!     verdict: "yes" | "tight" | "no",
//!     narrative: "<LLM explanation, or templated fallback>",
//!   }

use chrono::{DateTime, Datelike, Duration, NaiveDate, TimeZone, Utc};
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::PgPool;

use crate::config::Settings;
use crate::models::User;

#[derive(Debug, Clone, Serialize)]
pub struct SeriesPoint {
    pub month_index: i32,
    pub label: String,
    pub projected_balance_fen: i64,
    pub baseline_fen: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpcomingBill {
    pub subject: String,
    pub due_at: String,
    pub amount_fen: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ForecastOut {
    pub months: i32,
    pub starting_balance_fen: i64,
    pub avg_income_fen: i64,
    pub avg_expense_fen: i64,
    pub monthly_subscriptions_fen: i64,
    pub upcoming_bills: Vec<UpcomingBill>,
    pub series: Vec<SeriesPoint>,
    pub scenario: Value,
    pub verdict: String, // "yes" | "tight" | "no"
    pub narrative: String,
    /// Sum of money already held against future events (budget reservations,
    /// savings-goal holds). The forecast treats this as committed and uses it
    /// to deflate the "real" available balance.
    pub held_reservations_fen: i64,
    /// Sum of the active per-category monthly caps. Drives the
    /// "you'll blow your food budget at this rate" callout in the narrative.
    pub total_budget_limit_fen: i64,
    /// Per-budget headroom (limit - spent) so the UI can render the breach
    /// bars without re-querying.
    pub budgets: Vec<BudgetRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BudgetRow {
    pub category: String,
    pub limit_fen: i64,
    pub spent_fen: i64,
    pub remaining_fen: i64,
    pub breached: bool,
}

pub async fn available_balance_fen(pool: &PgPool, user_id: &str) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(available_balance_fen), 0)::BIGINT FROM accounts WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// Rolling 90-day income and expense averages in fen. The forecast
/// uses these as the steady-state cashflow.
pub async fn monthly_cashflow_fen(
    pool: &PgPool,
    user_id: &str,
) -> Result<(i64, i64), sqlx::Error> {
    let inc: (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(t.amount_fen), 0)::BIGINT FROM transactions t \
         JOIN accounts a ON a.id = t.account_id \
         WHERE a.user_id = $1 AND t.amount_fen > 0 \
         AND t.occurred_at >= NOW() - INTERVAL '90 days'",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    let exp: (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(-t.amount_fen), 0)::BIGINT FROM transactions t \
         JOIN accounts a ON a.id = t.account_id \
         WHERE a.user_id = $1 AND t.amount_fen < 0 \
         AND t.occurred_at >= NOW() - INTERVAL '90 days'",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    // The 90-day window is three months.
    Ok((inc.0 / 3, exp.0 / 3))
}

/// Sum the active subscriptions' expected monthly contribution.
pub async fn monthly_subscriptions_fen(pool: &PgPool, user_id: &str) -> Result<i64, sqlx::Error> {
    let rows: Vec<(i64, i32, String)> = sqlx::query_as(
        "SELECT estimated_amount_fen, interval_days, cancellation_status::text FROM subscriptions WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    let mut total: i64 = 0;
    for (amt, days, status) in rows {
        if status != "active" {
            continue;
        }
        let d = days.max(1) as i64;
        total += ((amt as f64) * 30.0 / d as f64) as i64;
    }
    Ok(total)
}

/// Pull any scheduled reminders in the forecast window. These represent
/// upcoming bills the user has already asked the assistant to remember.
pub async fn upcoming_bills_in_window(
    pool: &PgPool,
    user_id: &str,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<Vec<UpcomingBill>, sqlx::Error> {
    let rows: Vec<(String, String, DateTime<Utc>, Option<i64>)> = sqlx::query_as(
        "SELECT r.subject, r.kind::text, r.due_at, r.linked_subscription_id \
         FROM reminders r WHERE r.user_id = $1 AND r.status = 'scheduled' \
         AND r.due_at >= $2 AND r.due_at <= $3 \
         ORDER BY r.due_at ASC",
    )
    .bind(user_id)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;
    // Resolve amount from linked subscription if available.
    let mut out: Vec<UpcomingBill> = Vec::new();
    for (subject, _kind, due_at, linked_id) in rows {
        let amount_fen: i64 = if let Some(sub_id) = linked_id {
            let row: Option<(i64,)> =
                sqlx::query_as("SELECT estimated_amount_fen FROM subscriptions WHERE id = $1")
                    .bind(&sub_id)
                    .fetch_optional(pool)
                    .await?;
            row.map(|r| r.0).unwrap_or(0)
        } else {
            0
        };
        out.push(UpcomingBill {
            subject,
            due_at: due_at.to_rfc3339(),
            amount_fen,
        });
    }
    Ok(out)
}

fn month_label(d: NaiveDate) -> String {
    let month = match d.month() {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        _ => "Dec",
    };
    format!("{} {}", month, d.year())
}

/// Build a series of projected balances for the next `months` months.
/// The "baseline" series is what the balance would be WITHOUT the
/// scenario purchase. The "projected" series applies the scenario
/// spend at `spend_month_index` (0-based from the start of the
/// forecast window).
pub fn build_series(
    starting_balance: i64,
    avg_income: i64,
    avg_expense: i64,
    monthly_subs: i64,
    upcoming: &[UpcomingBill],
    months: i32,
    starting_month: NaiveDate,
    scenario_amount_fen: i64,
    spend_month_index: i32,
) -> Vec<SeriesPoint> {
    // Net monthly cashflow before subscriptions. Subscriptions are
    // pulled out so we can show them in the "monthly_subs" field
    // separately. If the user's transactions already include the
    // subscription charges, this would double-count — we conservatively
    // include subscriptions only as an additive, since the seed data
    // does not always have full subscription history.
    let net = avg_income - avg_expense;
    let mut series: Vec<SeriesPoint> = Vec::new();
    let mut bal = starting_balance;
    let mut baseline_bal = starting_balance;
    for i in 0..months {
        let month_date = add_months(starting_month, i);
        // Apply this month's upcoming bills.
        let mut month_bills: i64 = 0;
        for bill in upcoming {
            if let Ok(due) = DateTime::parse_from_rfc3339(&bill.due_at) {
                let due_utc = due.with_timezone(&Utc);
                if due_utc.date_naive().year() == month_date.year()
                    && due_utc.date_naive().month() == month_date.month()
                {
                    month_bills += bill.amount_fen;
                }
            }
        }
        bal += net - month_bills;
        baseline_bal += net - month_bills;
        if i == spend_month_index && scenario_amount_fen > 0 {
            bal -= scenario_amount_fen;
        }
        series.push(SeriesPoint {
            month_index: i,
            label: month_label(month_date),
            projected_balance_fen: bal,
            baseline_fen: baseline_bal,
        });
    }
    // Subscriptions are informational; they're already part of expense
    // in the user's transaction history, so we don't subtract them
    // twice. We still surface the number for the UI to display.
    let _ = monthly_subs;
    series
}

fn add_months(d: NaiveDate, n: i32) -> NaiveDate {
    let mut year = d.year();
    let mut month = d.month() as i32 + n;
    while month > 12 {
        month -= 12;
        year += 1;
    }
    while month < 1 {
        month += 12;
        year -= 1;
    }
    NaiveDate::from_ymd_opt(year, month as u32, d.day())
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(year, month as u32, 28).unwrap())
}

pub fn verdict(starting_balance: i64, end_balance: i64, scenario_amount_fen: i64) -> &'static str {
    // "yes"     → end balance comfortably above zero
    // "tight"   → end balance still positive but small
    // "no"      → end balance goes negative
    if end_balance < 0 {
        "no"
    } else if end_balance < (scenario_amount_fen / 2).max(50_000) {
        "tight"
    } else {
        "yes"
    }
}

fn first_of_next_month(now: DateTime<Utc>) -> NaiveDate {
    let y = now.year();
    let m = now.month();
    if m == 12 {
        NaiveDate::from_ymd_opt(y + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(y, m + 1, 1).unwrap()
    }
}

/// Sum of all held reservations (budget reservations + savings-goal holds
/// that aren't `released`/`consumed`). The forecast treats this as
/// committed cash and deflates the available balance by it.
pub async fn held_reservations_fen(
    pool: &PgPool,
    user_id: &str,
) -> Result<i64, sqlx::Error> {
    let res: (Option<i64>,) = sqlx::query_as(
        "SELECT COALESCE(SUM(amount_fen), 0)::BIGINT FROM reservations r \
         JOIN savings_goals g ON g.id = r.goal_id \
         WHERE g.user_id = $1 AND r.state = 'held'",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(res.0.unwrap_or(0))
}

/// Active per-category budget rows with current 30-day spend against each.
/// The forecast reports these so the LLM narrator can mention breach risk
/// when it answers "can I afford X?".
pub async fn active_budgets(
    pool: &PgPool,
    user_id: &str,
) -> Result<Vec<BudgetRow>, sqlx::Error> {
    let rows: Vec<crate::models::Budget> = sqlx::query_as(
        "SELECT * FROM budgets WHERE user_id = $1 ORDER BY category",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    let mut out = Vec::with_capacity(rows.len());
    for b in rows {
        let spend: (i64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(-t.amount_fen), 0)::BIGINT FROM transactions t \
             JOIN accounts a ON a.id = t.account_id \
             WHERE a.user_id = $1 AND t.category = $2 \
             AND ((CAST($3 AS TEXT) IS NULL) OR (a.id = $3)) \
             AND t.amount_fen < 0 AND t.occurred_at >= NOW() - INTERVAL '30 days'",
        )
        .bind(user_id)
        .bind(&b.category)
        .bind(b.account_id.as_deref())
        .fetch_one(pool)
        .await?;
        let limit = b.limit_fen;
        out.push(BudgetRow {
            category: b.category,
            limit_fen: limit,
            spent_fen: spend.0,
            remaining_fen: (limit - spend.0).max(0),
            breached: spend.0 > limit,
        });
    }
    Ok(out)
}

/// Top-level: run a forecast. Always returns a verdict + narrative; the
/// narrative is templated unless the LLM is reachable.
pub async fn run(
    pool: &PgPool,
    settings: &Settings,
    user: &User,
    amount_fen: i64,
    spend_month_index: i32,
    months: i32,
) -> Result<ForecastOut, sqlx::Error> {
    let months = months.clamp(1, 24);
    let spend_month_index = spend_month_index.clamp(0, months - 1);

    let starting = available_balance_fen(pool, &user.id).await?;
    let (avg_income, avg_expense) = monthly_cashflow_fen(pool, &user.id).await?;
    let monthly_subs = monthly_subscriptions_fen(pool, &user.id).await?;
    let held = held_reservations_fen(pool, &user.id).await?;
    let budgets = active_budgets(pool, &user.id).await?;
    let total_budget_limit: i64 = budgets.iter().map(|b| b.limit_fen).sum();

    let now = Utc::now();
    let window_start = first_of_next_month(now);
    let window_end = add_months(window_start, months);
    let window_end_dt = Utc
        .from_utc_datetime(&window_end.and_hms_opt(0, 0, 0).unwrap());
    let window_start_dt = Utc
        .from_utc_datetime(&window_start.and_hms_opt(0, 0, 0).unwrap());
    let upcoming = upcoming_bills_in_window(pool, &user.id, window_start_dt, window_end_dt).await?;

    let effective_starting = starting - held;
    let series = build_series(
        effective_starting,
        avg_income,
        avg_expense,
        monthly_subs,
        &upcoming,
        months,
        window_start,
        amount_fen,
        spend_month_index,
    );

    let end_balance = series.last().map(|p| p.projected_balance_fen).unwrap_or(effective_starting);
    let baseline_end = series.last().map(|p| p.baseline_fen).unwrap_or(effective_starting);
    let v = verdict(effective_starting, end_balance, amount_fen);

    let templated = match v {
        "no" => format!(
            "No — at this rate, your balance would drop to {} by month {}. \
             Consider waiting or saving toward the goal first.",
            fen_to_cny_str(end_balance),
            months
        ),
        "tight" => format!(
            "Tight — you can afford it, but the balance would land at {} which is below a one-month buffer.",
            fen_to_cny_str(end_balance)
        ),
        _ => format!(
            "Yes — you can afford it. After {} month(s), the balance is projected at {} \
             (vs. {} without the purchase).",
            months,
            fen_to_cny_str(end_balance),
            fen_to_cny_str(baseline_end)
        ),
    };

    let narrative = match narrate(settings, &series, amount_fen, starting, end_balance, v).await {
        Some(s) => s,
        None => templated,
    };

    Ok(ForecastOut {
        months,
        starting_balance_fen: starting,
        avg_income_fen: avg_income,
        avg_expense_fen: avg_expense,
        monthly_subscriptions_fen: monthly_subs,
        upcoming_bills: upcoming,
        series,
        scenario: json!({
            "amount_fen": amount_fen,
            "spend_month_index": spend_month_index,
            "as_of": now,
        }),
        verdict: v.to_string(),
        narrative,
        held_reservations_fen: held,
        total_budget_limit_fen: total_budget_limit,
        budgets,
    })
}

fn fen_to_cny_str(amount_fen: i64) -> String {
    let sign = if amount_fen < 0 { "-" } else { "" };
    let n = amount_fen.unsigned_abs();
    let yuan = n / 100;
    let fen = n % 100;
    format!("{}¥{}.{:02}", sign, yuan, fen)
}

async fn narrate(
    settings: &Settings,
    series: &[SeriesPoint],
    amount_fen: i64,
    starting: i64,
    end_balance: i64,
    verdict_str: &str,
) -> Option<String> {
    if !settings.unsloth_enabled() {
        return None;
    }
    let prompt = json!({
        "system": "You are BankPilot's forecasting narrator. Given a projected balance series and a one-word verdict, write a 2-3 sentence plain-language answer. Numbers must match exactly; do not invent any. Be specific about the month the user asked about.",
        "verdict": verdict_str,
        "amount_fen": amount_fen,
        "starting_balance_fen": starting,
        "end_balance_fen": end_balance,
        "series": series,
    });
    let body = serde_json::to_string(&prompt).ok()?;
    let url = format!("{}/chat/completions", settings.unsloth_url.trim_end_matches('/'));
    let req = crate::llm_http::client()
        .post(&url)
        .bearer_auth(&settings.unsloth_api_key)
        .json(&json!({
            "model": settings.unsloth_model,
            "messages": [{"role":"user","content": body}],
            "temperature": 0.3,
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

#[allow(dead_code)]
fn _silence(now: DateTime<Utc>, d: Duration) -> i64 {
    (now + d).timestamp()
}
