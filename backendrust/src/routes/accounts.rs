//! Accounts, transactions, and reports routes.

use actix_web::{web, HttpResponse};
use chrono::{DateTime, Duration, TimeZone, Utc};
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;

use crate::ctx::{AuthUser, RequestId};
use crate::errors::{self, ApiError};
use crate::models::{Account, Subscription, Transaction, User};

use crate::state::AppState;
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/accounts", web::get().to(list_accounts))
        .route("/accounts/{id}/balance", web::get().to(get_balance))
        .route("/transactions", web::get().to(list_transactions))
        .route("/reports/spending", web::get().to(report_spending))
        .route("/reports/anomalies", web::get().to(report_anomalies))
        .route("/reports/monthly", web::get().to(report_monthly))
        .route("/reports/annual", web::get().to(report_annual))
        .route("/reports/optimization", web::get().to(report_optimization));
}

async fn list_accounts(
    state: web::Data<AppState>,
    auth: AuthUser,
) -> Result<HttpResponse, ApiError> {
    let user = auth.0;
    let rows: Vec<Account> = sqlx::query_as(
        "SELECT * FROM accounts WHERE user_id = $1 ORDER BY created_at ASC",
    )
    .bind(&user.id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    let out: Vec<_> = rows
        .into_iter()
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
    Ok(HttpResponse::Ok().json(out))
}

async fn get_balance(
    state: web::Data<AppState>,
    auth: AuthUser,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let user = auth.0;
    let id = path.into_inner();
    let row: Option<Account> = sqlx::query_as("SELECT * FROM accounts WHERE id = $1")
        .bind(&id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    let a = row.ok_or_else(|| errors::not_found("Account not found."))?;
    if a.user_id != user.id {
        return Err(errors::not_found("Account not found."));
    }
    Ok(HttpResponse::Ok().json(json!({
        "id": a.id,
        "display_name": a.display_name,
        "masked_number": a.masked_number,
        "currency": a.currency,
        "available_balance_fen": a.available_balance_fen,
        "reserved_balance_fen": a.reserved_balance_fen,
    })))
}

#[derive(Deserialize)]
pub struct TxQuery {
    pub account_id: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub category: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}
fn default_limit() -> i64 {
    100
}

async fn list_transactions(
    state: web::Data<AppState>,
    auth: AuthUser,
    q: web::Query<TxQuery>,
) -> Result<HttpResponse, ApiError> {
    let user = auth.0;
    let q = q.into_inner();
    let mut sql = String::from(
        "SELECT t.* FROM transactions t JOIN accounts a ON a.id = t.account_id WHERE a.user_id = ?",
    );
    if q.account_id.is_some() {
        sql.push_str(" AND t.account_id = ?");
    }
    if q.from.is_some() {
        sql.push_str(" AND t.occurred_at >= ?");
    }
    if q.to.is_some() {
        sql.push_str(" AND t.occurred_at <= ?");
    }
    if q.category.is_some() {
        sql.push_str(" AND t.category = ?");
    }
    sql.push_str(" ORDER BY t.occurred_at DESC LIMIT ?");
    let mut query = sqlx::query_as::<_, Transaction>(&sql).bind(&user.id);
    if let Some(a) = q.account_id {
        query = query.bind(a);
    }
    if let Some(f) = q.from {
        query = query.bind(f);
    }
    if let Some(t) = q.to {
        query = query.bind(t);
    }
    if let Some(c) = q.category {
        query = query.bind(c);
    }
    query = query.bind(q.limit);
    let rows = query
        .fetch_all(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    let out: Vec<_> = rows
        .into_iter()
        .map(|t| {
            json!({
                "id": t.id,
                "account_id": t.account_id,
                "amount_fen": t.amount_fen,
                "direction": t.direction,
                "category": t.category,
                "merchant": t.merchant,
                "memo": t.memo,
                "occurred_at": t.occurred_at,
                "status": t.status,
            })
        })
        .collect();
    Ok(HttpResponse::Ok().json(out))
}

#[derive(Deserialize)]
pub struct SpendingQuery {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    #[serde(default = "default_group_by")]
    pub group_by: String,
}
fn default_group_by() -> String {
    "category".to_string()
}

async fn report_spending(
    state: web::Data<AppState>,
    auth: AuthUser,
    q: web::Query<SpendingQuery>,
) -> Result<HttpResponse, ApiError> {
    let user = auth.0;
    let q = q.into_inner();
    if !matches!(q.group_by.as_str(), "category" | "day") {
        return Err(errors::schema_invalid("group_by must be 'category' or 'day'"));
    }
    let now = Utc::now();
    let to = q.to.unwrap_or(now);
    let from = q.from.unwrap_or_else(|| to - Duration::days(30));
    let agg = aggregate(&state.pool, &user, from, to, &q.group_by).await?;
    Ok(HttpResponse::Ok().json(json!({
        "from_date": from,
        "to_date": to,
        "group_by": q.group_by,
        "income_fen": agg.0,
        "expense_fen": agg.1,
        "items": agg.2,
    })))
}

struct Agg(i64, i64, Vec<serde_json::Value>);

async fn aggregate(
    pool: &PgPool,
    user: &User,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    group_by: &str,
) -> Result<Agg, ApiError> {
    let rows: Vec<Transaction> = sqlx::query_as(
        "SELECT t.* FROM transactions t JOIN accounts a ON a.id = t.account_id WHERE a.user_id = $1 AND t.occurred_at >= $2 AND t.occurred_at <= $3",
    )
    .bind(&user.id)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await
    .map_err(|_| errors::internal_error())?;
    let mut income = 0i64;
    let mut expense = 0i64;
    let mut items_map: std::collections::BTreeMap<String, (i64, i64)> = std::collections::BTreeMap::new();
    for t in &rows {
        if t.amount_fen > 0 {
            income += t.amount_fen;
        } else if t.amount_fen < 0 {
            expense += -t.amount_fen;
        }
        if t.amount_fen < 0 {
            let key = if group_by == "category" {
                t.category.clone()
            } else {
                t.occurred_at.date_naive().to_string()
            };
            let entry = items_map.entry(key).or_insert((0, 0));
            entry.0 += -t.amount_fen;
            entry.1 += 1;
        }
    }
    let mut items: Vec<_> = items_map
        .into_iter()
        .map(|(cat, (total, count))| {
            let share = if expense > 0 {
                total as f64 / expense as f64 * 100.0
            } else {
                0.0
            };
            json!({
                "category": cat,
                "total_fen": total,
                "share_pct": share,
                "transaction_count": count,
            })
        })
        .collect();
    items.sort_by(|a, b| {
        let av = a.get("total_fen").and_then(|v| v.as_i64()).unwrap_or(0);
        let bv = b.get("total_fen").and_then(|v| v.as_i64()).unwrap_or(0);
        bv.cmp(&av)
    });
    Ok(Agg(income, expense, items))
}

async fn report_anomalies(
    state: web::Data<AppState>,
    auth: AuthUser,
    q: web::Query<SpendingQuery>,
) -> Result<HttpResponse, ApiError> {
    let user = auth.0;
    let q = q.into_inner();
    let now = Utc::now();
    let to = q.to.unwrap_or(now);
    let from = q.from.unwrap_or_else(|| to - Duration::days(60));
    let rows: Vec<Transaction> = sqlx::query_as(
        "SELECT t.* FROM transactions t JOIN accounts a ON a.id = t.account_id WHERE a.user_id = $1 AND t.occurred_at >= $2 AND t.occurred_at <= $3 ORDER BY t.occurred_at ASC",
    )
    .bind(&user.id)
    .bind(from)
    .bind(to)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    let all: Vec<Transaction> = sqlx::query_as(
        "SELECT t.* FROM transactions t JOIN accounts a ON a.id = t.account_id WHERE a.user_id = $1 AND t.amount_fen < 0 ORDER BY t.occurred_at ASC",
    )
    .bind(&user.id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    let mut by_merchant: std::collections::BTreeMap<String, Vec<Transaction>> =
        std::collections::BTreeMap::new();
    for t in all {
        by_merchant.entry(t.merchant.clone()).or_default().push(t);
    }
    let mut out: Vec<serde_json::Value> = vec![];
    for t in rows {
        if t.amount_fen >= 0 {
            continue;
        }
        let amount = -t.amount_fen;
        let mut reasons: Vec<String> = vec![];
        let mut baseline: Option<i64> = None;
        if let Some(history) = by_merchant.get(&t.merchant) {
            let amounts: Vec<i64> = history.iter().map(|x| -x.amount_fen).collect();
            if amounts.len() >= 3 {
                let mut sorted = amounts.clone();
                sorted.sort();
                baseline = Some(sorted[sorted.len() / 2]);
                if let Some(b) = baseline {
                    if b > 0 && amount > 3 * b {
                        reasons.push(format!("amount > 3× median ({b} fen)"));
                    }
                }
            }
            if history.len() == 1 && amount > 5000 {
                reasons.push("first-time merchant with non-trivial amount".to_string());
            }
            let recent: Vec<&Transaction> = history
                .iter()
                .filter(|x| x.id != t.id && (x.occurred_at - t.occurred_at).num_days().abs() < 7)
                .collect();
            if recent
                .iter()
                .any(|x| ((-x.amount_fen) - amount).abs() < std::cmp::max(1, amount / 10))
            {
                reasons.push("similar amount at same merchant within 7 days".to_string());
            }
        }
        if !reasons.is_empty() {
            out.push(json!({
                "transaction_id": t.id,
                "merchant": t.merchant,
                "amount_fen": -t.amount_fen,
                "occurred_at": t.occurred_at,
                "reasons": reasons,
                "baseline_median_fen": baseline,
            }));
        }
    }
    Ok(HttpResponse::Ok().json(out))
}

#[derive(Deserialize)]
pub struct MonthlyQuery {
    pub year: i32,
    pub month: i32,
}

async fn report_monthly(
    state: web::Data<AppState>,
    auth: AuthUser,
    q: web::Query<MonthlyQuery>,
) -> Result<HttpResponse, ApiError> {
    if !(1..=12).contains(&q.month) {
        return Err(errors::schema_invalid("month must be 1..12"));
    }
    let user = auth.0;
    let start = chrono::Utc
        .with_ymd_and_hms(q.year, q.month as u32, 1, 0, 0, 0)
        .single()
        .ok_or_else(|| errors::schema_invalid("invalid year/month"))?;
    let (next_year, next_month) = if q.month == 12 {
        (q.year + 1, 1)
    } else {
        (q.year, q.month + 1)
    };
    let end = chrono::Utc
        .with_ymd_and_hms(next_year, next_month as u32, 1, 0, 0, 0)
        .single()
        .unwrap();
    let agg = aggregate(&state.pool, &user, start, end, "category").await?;
    let (prev_year, prev_month) = if q.month == 1 {
        (q.year - 1, 12)
    } else {
        (q.year, q.month - 1)
    };
    let prev_start = chrono::Utc
        .with_ymd_and_hms(prev_year, prev_month as u32, 1, 0, 0, 0)
        .single()
        .unwrap();
    let prev = aggregate(&state.pool, &user, prev_start, start, "category").await?;
    let has_baseline = prev.0 > 0 || prev.1 > 0;
    let mom_delta = if has_baseline { Some(agg.1 - prev.1) } else { None };
    Ok(HttpResponse::Ok().json(json!({
        "year": q.year,
        "month": q.month,
        "income_fen": agg.0,
        "expense_fen": agg.1,
        "by_category": agg.2,
        "mom_delta_fen": mom_delta,
        "has_baseline": has_baseline,
    })))
}

#[derive(Deserialize)]
pub struct AnnualQuery {
    pub year: i32,
}

async fn report_annual(
    state: web::Data<AppState>,
    auth: AuthUser,
    q: web::Query<AnnualQuery>,
) -> Result<HttpResponse, ApiError> {
    let user = auth.0;
    let start = chrono::Utc
        .with_ymd_and_hms(q.year, 1, 1, 0, 0, 0)
        .single()
        .ok_or_else(|| errors::schema_invalid("invalid year"))?;
    let end = chrono::Utc
        .with_ymd_and_hms(q.year + 1, 1, 1, 0, 0, 0)
        .single()
        .unwrap();
    let agg = aggregate(&state.pool, &user, start, end, "category").await?;
    let earliest: Option<Transaction> = sqlx::query_as(
        "SELECT t.* FROM transactions t JOIN accounts a ON a.id = t.account_id WHERE a.user_id = $1 AND t.occurred_at >= $2 AND t.occurred_at < $3 ORDER BY t.occurred_at ASC LIMIT 1",
    )
    .bind(&user.id)
    .bind(start)
    .bind(end)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    let missing = if let Some(ref e) = earliest {
        if e.occurred_at > start {
            Some(format!(
                "Year {} coverage begins on {}; earlier days are not in this dataset.",
                q.year,
                e.occurred_at.date_naive()
            ))
        } else {
            None
        }
    } else {
        None
    };
    Ok(HttpResponse::Ok().json(json!({
        "year": q.year,
        "income_fen": agg.0,
        "expense_fen": agg.1,
        "by_category": agg.2,
        "coverage_from": earliest.as_ref().map(|t| t.occurred_at),
        "coverage_to": end,
        "missing_coverage_notice": missing,
    })))
}

// --- Optimization report ------------------------------------------------
//
// `GET /api/v1/reports/optimization` — ranked, actionable suggestions
// for reducing cost and optimising cashflow. Built from the user's
// transactions, subscriptions, and accounts. All amounts are integer
// fen so the UI can `formatCny` without rounding. Suggestions are
// sorted by `monthly_savings_fen` descending so the "biggest wins" land
// at the top of the UI list.
//
// Each suggestion is a `serde_json::Value` so we can attach a
// hand-rolled set of fields per kind (the field set differs per code).
// Common fields:
//
//   code            "unused_subscription" | "subscription_bloat" |
//                   "category_overspend"  | "low_balance_risk"    |
//                   "fee_consolidation"   | "transfer_fragmentation" |
//                   "set_autopay"
//   severity        "info" | "warning" | "critical"
//   title           short headline
//   body            one-sentence explanation
//   monthly_savings_fen    integer fen, 0 when qualitative
//   annual_savings_fen     monthly_savings_fen * 12 (or 0)
//   action_label    button text (e.g. "Cancel", "Review")
//   action_endpoint optional deep-link, e.g. "/subscriptions"
//   evidence        { merchant, amount_fen, count, … } debug aid
//   rank            1-based ordering in the response list

async fn report_optimization(
    state: web::Data<AppState>,
    auth: AuthUser,
) -> Result<HttpResponse, ApiError> {
    let user = auth.0;
    let pool = &state.pool;
    let now = Utc::now();

    // Pull the three primary data sources in parallel. We do it
    // sequentially because sqlx on a single connection is fine for
    // this small read-only query, and keeping it serial makes the
    // lifetimes easier to reason about.
    let accounts: Vec<Account> = sqlx::query_as(
        "SELECT * FROM accounts WHERE user_id = $1 ORDER BY created_at ASC",
    )
    .bind(&user.id)
    .fetch_all(pool)
    .await
    .map_err(|_| errors::internal_error())?;
    let subscriptions: Vec<Subscription> = sqlx::query_as(
        "SELECT id, user_id, merchant, estimated_amount_fen, interval_days, next_expected_at, last_charge_at, evidence::text AS evidence, cancellation_status, cancelled_at, created_at \
         FROM subscriptions WHERE user_id = $1",
    )
    .bind(&user.id)
    .fetch_all(pool)
    .await
    .map_err(|_| errors::internal_error())?;
    // Last 90 days of debit transactions for pattern detection.
    let tx_90d: Vec<Transaction> = sqlx::query_as(
        "SELECT t.* FROM transactions t JOIN accounts a ON a.id = t.account_id \
         WHERE a.user_id = $1 AND t.amount_fen < 0 \
           AND t.occurred_at >= NOW() - INTERVAL '90 days' \
         ORDER BY t.occurred_at DESC",
    )
    .bind(&user.id)
    .fetch_all(pool)
    .await
    .map_err(|_| errors::internal_error())?;
    let tx_30d: Vec<&Transaction> = tx_90d
        .iter()
        .filter(|t| t.occurred_at >= now - Duration::days(30))
        .collect();

    let mut out: Vec<serde_json::Value> = Vec::new();

    // --- 1. Unused subscription (cancellable) ---------------------------
    //
    // A subscription is "unused" if it has been auto-detected and is
    // still `active`, but the user has not made any matching merchant
    // transaction in the last 30 days — meaning they're paying for
    // something they don't use. (For subscriptions whose `interval_days`
    // > 30 the user is still paying even if the last charge was older.)
    let active_subs: Vec<&Subscription> = subscriptions
        .iter()
        .filter(|s| s.cancellation_status == "active")
        .collect();
    for s in &active_subs {
        let last = s.last_charge_at.unwrap_or(s.created_at);
        let days_since_charge = (now - last).num_days();
        let any_recent = tx_30d.iter().any(|t| {
            t.merchant.eq_ignore_ascii_case(&s.merchant) && t.amount_fen.abs() <= (s.estimated_amount_fen * 3 / 2).max(s.estimated_amount_fen)
        });
        // "Unused" = interval > 30d AND last charge > 60d ago, OR any
        // active sub whose merchant hasn't appeared in 30d of txns.
        let unused_long_window =
            s.interval_days as i64 > 30 && days_since_charge > (s.interval_days as i64 + 5);
        let unused_30d = !any_recent && days_since_charge > 30;
        if unused_long_window || unused_30d {
            // Monthly equivalent: pro-rate the annual cost over 12.
            let monthly_fen =
                (s.estimated_amount_fen as f64 * 30.0 / s.interval_days.max(1) as f64) as i64;
            out.push(json!({
                "code": "unused_subscription",
                "severity": "warning",
                "title": format!("You may not need '{}' anymore", s.merchant),
                "body": format!(
                    "No charges from '{}' in the last {} days. Cancel to save about {} per month.",
                    s.merchant,
                    days_since_charge,
                    crate::money::fen_to_cny_str(monthly_fen)
                ),
                "monthly_savings_fen": monthly_fen,
                "annual_savings_fen": monthly_fen * 12,
                "action_label": "Review & cancel",
                "action_endpoint": "/subscriptions",
                "evidence": {
                    "subscription_id": s.id,
                    "merchant": s.merchant,
                    "estimated_amount_fen": s.estimated_amount_fen,
                    "interval_days": s.interval_days,
                    "last_charge_at": s.last_charge_at,
                    "days_since_charge": days_since_charge,
                }
            }));
        }
    }

    // --- 2. Subscription bloat ----------------------------------------
    if active_subs.len() >= 3 {
        let monthly_sub_total: i64 = active_subs
            .iter()
            .map(|s| {
                (s.estimated_amount_fen as f64 * 30.0 / s.interval_days.max(1) as f64) as i64
            })
            .sum();
        // Sort to pick the median-priced sub as the "cut one" target.
        let mut prices: Vec<i64> = active_subs
            .iter()
            .map(|s| {
                (s.estimated_amount_fen as f64 * 30.0 / s.interval_days.max(1) as f64) as i64
            })
            .collect();
        prices.sort_unstable();
        let median = prices[prices.len() / 2];
        out.push(json!({
            "code": "subscription_bloat",
            "severity": "info",
            "title": format!("You have {} active subscriptions", active_subs.len()),
            "body": format!(
                "Total recurring cost is about {} per month. Most users keep 1–2; cancelling the median one (~{}) could trim your bill by ~20%.",
                crate::money::fen_to_cny_str(monthly_sub_total),
                crate::money::fen_to_cny_str(median)
            ),
            "monthly_savings_fen": median / 5, // 20% trim
            "annual_savings_fen": (median / 5) * 12,
            "action_label": "Audit subscriptions",
            "action_endpoint": "/subscriptions",
            "evidence": {
                "active_count": active_subs.len(),
                "merchants": active_subs.iter().map(|s| &s.merchant).collect::<Vec<_>>(),
                "monthly_total_fen": monthly_sub_total,
            }
        }));
    }

    // --- 3. Category overspend ----------------------------------------
    //
    // For the last 30 days, if any single category consumes > 40% of
    // the total spend, surface a budgeting tip. We use 10% of the
    // category as the projected monthly savings — a friendly nudge,
    // not a guarantee.
    let mut by_cat_30d: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();
    let mut total_30d: i64 = 0;
    for t in &tx_30d {
        let amt = t.amount_fen.abs();
        total_30d += amt;
        *by_cat_30d.entry(t.category.clone()).or_insert(0) += amt;
    }
    if total_30d > 0 {
        for (cat, amt) in &by_cat_30d {
            let share = (*amt as f64) / (total_30d as f64);
            if share > 0.40 && *amt > 50_000 {
                // ignore tiny totals to avoid noise
                let projected_savings = (*amt / 10).max(0);
                // Suggest a soft cap that's ~10% below current spend so the
                // user gets headroom while still feeling the constraint.
                let suggested_limit_fen = (*amt * 9 / 10).max(10_000);
                out.push(json!({
                    "code": "category_overspend",
                    "severity": "info",
                    "title": format!("'{cat}' is {}% of your spending this month", (share * 100.0).round() as i64),
                    "body": format!(
                        "You spent {} on {} in the last 30 days. Setting a soft cap or moving 10% to a goal saves about {} a month.",
                        crate::money::fen_to_cny_str(*amt),
                        cat,
                        crate::money::fen_to_cny_str(projected_savings)
                    ),
                    "monthly_savings_fen": projected_savings,
                    "annual_savings_fen": projected_savings * 12,
                    "action_label": "Set a budget",
                    // Hand-off: deep-link into /budgets with the over-spending
                    // category pre-filled. The dashboard modal reads
                    // `?category=…&limit_fen=…` and seeds the form.
                    "action_endpoint": format!(
                        "/budgets?category={}&limit_fen={}",
                        urlencoding::encode(cat),
                        suggested_limit_fen
                    ),
                    "evidence": {
                        "category": cat,
                        "amount_fen": amt,
                        "total_fen": total_30d,
                        "share_pct": ((share * 1000.0).round() as i64) as f64 / 10.0,
                        "suggested_limit_fen": suggested_limit_fen,
                    }
                }));
            }
        }
    }

    // --- 3b. Active budget breach / near-breach -----------------------
    // Pull the user's budgets and flag any that are over or close to over.
    let budgets: Vec<crate::models::Budget> = sqlx::query_as(
        "SELECT * FROM budgets WHERE user_id = $1"
    )
    .bind(&user.id)
    .fetch_all(pool)
    .await
    .map_err(|_| errors::internal_error())?;
    for b in &budgets {
        let spend_30d: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(-t.amount_fen), 0)::BIGINT FROM transactions t \
             JOIN accounts a ON a.id = t.account_id \
             WHERE a.user_id = $1 AND t.category = $2 \
             AND ((CAST($3 AS TEXT) IS NULL) OR (a.id = $3)) \
             AND t.amount_fen < 0 AND t.occurred_at >= NOW() - INTERVAL '30 days'",
        )
        .bind(&user.id)
        .bind(&b.category)
        .bind(b.account_id.as_deref())
        .fetch_one(pool)
        .await
        .map_err(|_| errors::internal_error())?;
        if spend_30d > b.limit_fen {
            out.push(json!({
                "code": "budget_breached",
                "severity": "warning",
                "title": format!("You're over your '{}' budget", b.category),
                "body": format!(
                    "You've spent {} against a {} cap this month. Tighten the cap, move the rest to a goal, or roll some spend into next month.",
                    crate::money::fen_to_cny_str(spend_30d),
                    crate::money::fen_to_cny_str(b.limit_fen)
                ),
                "monthly_savings_fen": spend_30d - b.limit_fen,
                "annual_savings_fen": (spend_30d - b.limit_fen) * 12,
                "action_label": "Review budget",
                "action_endpoint": format!("/budgets?focus={}", urlencoding::encode(&b.id)),
                "evidence": {
                    "budget_id": b.id,
                    "category": b.category,
                    "limit_fen": b.limit_fen,
                    "spent_fen": spend_30d,
                }
            }));
        } else if b.limit_fen > 0 && (spend_30d as f64) / (b.limit_fen as f64) >= 0.8 {
            out.push(json!({
                "code": "budget_near_limit",
                "severity": "info",
                "title": format!("Close to your '{}' cap", b.category),
                "body": format!(
                    "{} of {} used. Worth slowing down for the rest of the month.",
                    crate::money::fen_to_cny_str(spend_30d),
                    crate::money::fen_to_cny_str(b.limit_fen)
                ),
                "monthly_savings_fen": 0,
                "annual_savings_fen": 0,
                "action_label": "Review budget",
                "action_endpoint": format!("/budgets?focus={}", urlencoding::encode(&b.id)),
                "evidence": {
                    "budget_id": b.id,
                    "category": b.category,
                    "limit_fen": b.limit_fen,
                    "spent_fen": spend_30d,
                }
            }));
        }
    }

    // --- 4. Low-balance risk ------------------------------------------
    //
    // If the user's available cash across primary accounts is less than
    // 1.5× their average monthly expense, surface a critical warning.
    let monthly_expense_avg: i64 = if total_30d > 0 { total_30d } else { 0 };
    let available: i64 = accounts
        .iter()
        .map(|a| a.available_balance_fen)
        .sum();
    if monthly_expense_avg > 0 && available < monthly_expense_avg + (monthly_expense_avg / 2) {
        out.push(json!({
            "code": "low_balance_risk",
            "severity": "critical",
            "title": "Low runway — your cushion is thin".to_string(),
            "body": format!(
                "Available balance ({}) is below 1.5× your typical monthly spend ({}). Move some cash to checking, or pause discretionary subscriptions.",
                crate::money::fen_to_cny_str(available),
                crate::money::fen_to_cny_str(monthly_expense_avg),
            ),
            "monthly_savings_fen": 0,
            "annual_savings_fen": 0,
            "action_label": "Move money to checking",
            "action_endpoint": "/dashboard",
            "evidence": {
                "available_balance_fen": available,
                "avg_monthly_expense_fen": monthly_expense_avg,
            }
        }));
    }

    // --- 5. Transfer fragmentation (many small transfers to one payee) ----
    //
    // More than 3 transfers to the same payee in 30 days, none of them
    // scheduled — group into a single scheduled or split-bill.
    let mut by_payee: std::collections::BTreeMap<(String, String), Vec<&Transaction>> =
        std::collections::BTreeMap::new();
    for t in &tx_30d {
        if t.category != "transfer" {
            continue;
        }
        // No payee_id on `transactions` — key by (merchant, account_id) as a
        // reasonable proxy.
        by_payee
            .entry((t.merchant.clone(), t.account_id.clone()))
            .or_default()
            .push(t);
    }
    for ((merchant, _account), txns) in &by_payee {
        if txns.len() < 4 {
            continue;
        }
        let total: i64 = txns.iter().map(|t| t.amount_fen.abs()).sum();
        out.push(json!({
            "code": "transfer_fragmentation",
            "severity": "info",
            "title": format!("You sent {} to '{}' this month — in {} small transfers", crate::money::fen_to_cny_str(total), merchant, txns.len()),
            "body": format!(
                "Consolidate the {} small transfers to '{}' into a single scheduled payment or split-bill. Less friction, same outcome.",
                txns.len(),
                merchant,
            ),
            "monthly_savings_fen": 0,
            "annual_savings_fen": 0,
            "action_label": "Consolidate",
            "action_endpoint": "/transfers",
            "evidence": {
                "merchant": merchant,
                "count": txns.len(),
                "total_fen": total,
            }
        }));
    }

    // --- 6. High-frequency micro purchases -----------------------------
    //
    // More than 10 transactions < ¥5 in the last 30d. Tells the user
    // they may be losing more to small daily purchases than they
    // think.
    let micro: Vec<&&Transaction> = tx_30d
        .iter()
        .filter(|t| t.amount_fen.abs() < 500)
        .collect();
    if micro.len() >= 10 {
        let total_micro: i64 = micro.iter().map(|t| t.amount_fen.abs()).sum();
        out.push(json!({
            "code": "micro_spend",
            "severity": "info",
            "title": format!("{} small purchases in 30 days", micro.len()),
            "body": format!(
                "You made {} charges under ¥5 in the last 30 days, totalling {}. A small daily limit can recover that cash.",
                micro.len(),
                crate::money::fen_to_cny_str(total_micro),
            ),
            "monthly_savings_fen": total_micro / 4, // 25% trim
            "annual_savings_fen": (total_micro / 4) * 12,
            "action_label": "Add a daily cap",
            "action_endpoint": "/accounts",
            "evidence": {
                "count": micro.len(),
                "total_fen": total_micro,
            }
        }));
    }

    // --- Sort + rank ---------------------------------------------------
    // Highest monthly_savings_fen first. Ties broken by severity
    // (critical > warning > info).
    let sev_weight = |s: &str| match s {
        "critical" => 3_i64,
        "warning" => 2,
        _ => 1,
    };
    out.sort_by(|a, b| {
        let av = a.get("monthly_savings_fen").and_then(|v| v.as_i64()).unwrap_or(0);
        let bv = b.get("monthly_savings_fen").and_then(|v| v.as_i64()).unwrap_or(0);
        let asev = sev_weight(a.get("severity").and_then(|v| v.as_str()).unwrap_or("info"));
        let bsev = sev_weight(b.get("severity").and_then(|v| v.as_str()).unwrap_or("info"));
        bv.cmp(&av)
            .then_with(|| bsev.cmp(&asev))
            .then_with(|| a.get("code").and_then(|v| v.as_str()).unwrap_or("")
                .cmp(b.get("code").and_then(|v| v.as_str()).unwrap_or("")))
    });
    let total_monthly_savings: i64 = out
        .iter()
        .map(|s| s.get("monthly_savings_fen").and_then(|v| v.as_i64()).unwrap_or(0))
        .sum();
    let ranked: Vec<serde_json::Value> = out
        .into_iter()
        .enumerate()
        .map(|(i, mut v)| {
            if let Some(map) = v.as_object_mut() {
                map.insert("rank".into(), json!(i + 1));
            }
            v
        })
        .collect();

    Ok(HttpResponse::Ok().json(json!({
        "generated_at": now.to_rfc3339(),
        "subscription_count": active_subs.len(),
        "total_monthly_savings_fen": total_monthly_savings,
        "total_annual_savings_fen": total_monthly_savings * 12,
        "suggestions": ranked,
    })))
}
