//! Spending statement download route.
//!
//! `GET /api/v1/reports/statement?month=YYYY-MM&format=csv|tsv|json`
//!
//! Returns a per-account, per-transaction statement for the requested
//! month (defaults to the current calendar month in `Asia/Shanghai`
//! offset — same as the rest of the demo's "today"). The response is
//! served with `Content-Disposition: attachment` so the browser opens
//! the platform's "save file" dialog without the user clicking
//! anything else. This is what the agent chat surfaces as
//! `statement_url` when the user asks "show my spending this month".
//!
//! Format choices:
//! - `csv`  — quoted CSV, opens in Excel/Sheets, `text/csv` MIME.
//! - `tsv`  — tab-separated, `text/tab-separated-values`, easier to
//!            paste into Notion / Slack.
//! - `json` — machine-readable, `application/json`.
//!
//! Money is integer fen in the CSV/TSV columns (`amount_fen`) AND a
//! derived `amount_cny` string column so spreadsheet users don't have
//! to do the divide. Negative amounts are debits (spends); positive
//! are credits (refunds, income). Totals are in the trailer.

use actix_web::{web, HttpRequest, HttpResponse};
use chrono::{DateTime, Datelike, Duration, NaiveDate, TimeZone, Utc};
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;

use crate::ctx::AuthUser;
use crate::errors::{self, ApiError};
use crate::models::Account;
use crate::money::fen_to_cny_str;
use crate::state::AppState;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/reports/statement", web::get().to(get_statement));
}

#[derive(Deserialize)]
pub struct StatementQuery {
    /// `YYYY-MM`. Defaults to the current month in `Utc` if absent.
    pub month: Option<String>,
    /// `csv` (default) | `tsv` | `json`.
    pub format: Option<String>,
    /// Optional account filter — when present, only include transactions
    /// for that account. The user must own the account or we 404.
    pub account_id: Option<String>,
}

/// Resolve a `YYYY-MM` string to the (start, end) UTC instants for that
/// calendar month, inclusive of the start and exclusive of the end. When
/// `month` is missing or malformed, returns the current month.
fn month_window(month: Option<&str>) -> (DateTime<Utc>, DateTime<Utc>) {
    let now = Utc::now();
    let (y, m) = match month {
        Some(s) => match parse_month(s) {
            Some(ym) => ym,
            None => (now.year(), now.month()),
        },
        None => (now.year(), now.month()),
    };
    let start = NaiveDate::from_ymd_opt(y, m, 1)
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|d| Utc.from_utc_datetime(&d))
        .unwrap_or(now);
    let end = if m == 12 {
        NaiveDate::from_ymd_opt(y + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(y, m + 1, 1)
    }
    .and_then(|d| d.and_hms_opt(0, 0, 0))
    .map(|d| Utc.from_utc_datetime(&d))
    .unwrap_or(now + Duration::days(30));
    (start, end)
}

fn parse_month(s: &str) -> Option<(i32, u32)> {
    // Accept "YYYY-MM" and "YYYY-MM-DD" (use the first 7 chars).
    let s = s.trim();
    if s.len() < 7 {
        return None;
    }
    let y: i32 = s.get(0..4)?.parse().ok()?;
    if s.as_bytes().get(4)? != &b'-' {
        return None;
    }
    let m: u32 = s.get(5..7)?.parse().ok()?;
    if !(1..=12).contains(&m) {
        return None;
    }
    Some((y, m))
}

async fn get_statement(
    state: web::Data<AppState>,
    auth: AuthUser,
    req: HttpRequest,
    q: web::Query<StatementQuery>,
) -> Result<HttpResponse, ApiError> {
    let user = auth.0;
    let q = q.into_inner();
    let (start, end) = month_window(q.month.as_deref());
    let format = q.format.as_deref().unwrap_or("csv").to_ascii_lowercase();
    if !matches!(format.as_str(), "csv" | "tsv" | "json") {
        return Err(errors::bad_request("format must be csv, tsv, or json."));
    }

    // Per-account metadata for the header block.
    let mut accounts: Vec<Account> = sqlx::query_as(
        "SELECT * FROM accounts WHERE user_id = $1 ORDER BY created_at ASC",
    )
    .bind(&user.id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    if let Some(aid) = &q.account_id {
        if !accounts.iter().any(|a| &a.id == aid) {
            return Err(errors::not_found("Account not found."));
        }
        accounts.retain(|a| &a.id == aid);
    }

    // Transaction rows for the window. The `account_id` filter is folded
    // into the SQL; when omitted, all of the user's accounts are included.
    let rows = fetch_rows(
        &state.pool,
        &user.id,
        q.account_id.as_deref(),
        start,
        end,
    )
    .await
    .map_err(|_| errors::internal_error())?;

    // Aggregate totals.
    let total_spend_fen: i64 = rows.iter().filter(|r| r.amount_fen < 0).map(|r| -r.amount_fen).sum();
    let total_income_fen: i64 = rows.iter().filter(|r| r.amount_fen > 0).map(|r| r.amount_fen).sum();
    let txn_count = rows.len();

    // Filename: bankpilot-statement-YYYY-MM.<ext>
    let month_tag = format!("{:04}-{:02}", start.year(), start.month());
    let ext = match format.as_str() {
        "tsv" => "tsv",
        "json" => "json",
        _ => "csv",
    };
    let filename = format!("bankpilot-statement-{month_tag}.{ext}");

    match format.as_str() {
        "json" => {
            let payload = json!({
                "user": {
                    "id": user.id,
                    "display_name": user.display_name,
                },
                "month": month_tag,
                "period": {
                    "start": start.to_rfc3339(),
                    "end": end.to_rfc3339(),
                },
                "accounts": accounts.iter().map(|a| json!({
                    "id": a.id,
                    "display_name": a.display_name,
                    "masked_number": a.masked_number,
                    "currency": a.currency,
                    "available_balance_fen": a.available_balance_fen,
                    "reserved_balance_fen": a.reserved_balance_fen,
                })).collect::<Vec<_>>(),
                "totals": {
                    "spend_fen": total_spend_fen,
                    "spend_cny": fen_to_cny_str(total_spend_fen),
                    "income_fen": total_income_fen,
                    "income_cny": fen_to_cny_str(total_income_fen),
                    "transaction_count": txn_count,
                },
                "transactions": rows.iter().map(|r| json!({
                    "id": r.id,
                    "account_id": r.account_id,
                    "occurred_at": r.occurred_at,
                    "merchant": r.merchant,
                    "category": r.category,
                    "direction": r.direction,
                    "amount_fen": r.amount_fen,
                    "amount_cny": fen_to_cny_str(r.amount_fen),
                    "memo": r.memo,
                    "status": r.status,
                })).collect::<Vec<_>>(),
            });
            Ok(HttpResponse::Ok()
                .content_type("application/json; charset=utf-8")
                .insert_header(("Content-Disposition", format!("attachment; filename=\"{filename}\"")))
                .insert_header(("X-Statement-Month", month_tag.clone()))
                .insert_header(("X-Statement-Transaction-Count", txn_count.to_string()))
                .json(payload))
        }
        "csv" | "tsv" => {
            let sep = if format == "tsv" { '\t' } else { ',' };
            let nl = "\r\n";
            let mut out = String::new();
            // Header / cover rows (commented for CSV with `# `, ignored
            // by spreadsheet importers that just skip non-comma leading
            // text on line 1; for TSV the same `#` works as a human hint).
            out.push_str(&format!("# BankPilot statement — {month_tag}{nl}"));
            out.push_str(&format!("# Account holder: {}{nl}", user.display_name));
            out.push_str(&format!("# Period: {} → {}{nl}", start.format("%Y-%m-%d"), (end - Duration::seconds(1)).format("%Y-%m-%d")));
            out.push_str(&format!("# Generated: {}{nl}", Utc::now().format("%Y-%m-%dT%H:%M:%SZ")));
            out.push_str(&format!("# Transaction count: {txn_count}{nl}"));
            out.push_str(&format!("# Total spend: {} ({} fen){nl}", fen_to_cny_str(total_spend_fen), total_spend_fen));
            out.push_str(&format!("# Total income: {} ({} fen){nl}", fen_to_cny_str(total_income_fen), total_income_fen));
            out.push_str(&format!("# Accounts:{}", nl));
            for a in &accounts {
                out.push_str(&format!(
                    "#   - {} {} (available {}){nl}",
                    a.display_name,
                    a.masked_number,
                    fen_to_cny_str(a.available_balance_fen)
                ));
            }
            out.push_str(nl);
            // Column header.
            out.push_str(&format!(
                "occurred_at{sep}account_id{sep}direction{sep}amount_fen{sep}amount_cny{sep}category{sep}merchant{sep}memo{sep}status{sep}tx_id{nl}"
            ));
            // Rows.
            for r in &rows {
                out.push_str(&format!(
                    "{ts}{sep}{aid}{sep}{dir}{sep}{fen}{sep}{cny}{sep}{cat}{sep}{m}{sep}{memo}{sep}{st}{sep}{tid}{nl}",
                    ts = r.occurred_at.format("%Y-%m-%dT%H:%M:%SZ"),
                    aid = r.account_id,
                    dir = r.direction,
                    fen = r.amount_fen,
                    cny = csv_field(&fen_to_cny_str(r.amount_fen), sep),
                    cat = csv_field(&r.category, sep),
                    m = csv_field(&r.merchant, sep),
                    memo = csv_field(&r.memo, sep),
                    st = r.status,
                    tid = r.id,
                ));
            }
            // Trailer.
            out.push_str(nl);
            out.push_str(&format!("# Totals{nl}"));
            // The trailer is informational; emit it on a single line per
            // metric using the chosen separator. `s = sep, fen = …` is
            // wrapped in `format!` arg style.
            let spend_line = format!("# spend_fen{sep}{fen}", sep = sep, fen = total_spend_fen);
            let income_line = format!("# income_fen{sep}{fen}", sep = sep, fen = total_income_fen);
            out.push_str(&spend_line);
            out.push_str(nl);
            out.push_str(&income_line);
            out.push_str(nl);
            let mime = if format == "tsv" { "text/tab-separated-values" } else { "text/csv" };
            Ok(HttpResponse::Ok()
                .content_type(format!("{mime}; charset=utf-8"))
                .insert_header(("Content-Disposition", format!("attachment; filename=\"{filename}\"")))
                .insert_header(("X-Statement-Month", month_tag.clone()))
                .insert_header(("X-Statement-Transaction-Count", txn_count.to_string()))
                .body(out))
        }
        _ => unreachable!("format guard above"),
    }
}

fn csv_field(s: &str, sep: char) -> String {
    if sep == ',' && (s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r')) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct Row {
    id: String,
    account_id: String,
    direction: String,
    amount_fen: i64,
    category: String,
    merchant: String,
    memo: String,
    occurred_at: DateTime<Utc>,
    status: String,
}

async fn fetch_rows(
    pool: &PgPool,
    user_id: &str,
    account_id: Option<&str>,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<Vec<Row>, sqlx::Error> {
    if let Some(aid) = account_id {
        sqlx::query_as(
            "SELECT t.id, t.account_id, t.direction, t.amount_fen, t.category, t.merchant, t.memo, t.occurred_at, t.status \
             FROM transactions t JOIN accounts a ON a.id = t.account_id \
             WHERE a.user_id = $1 AND t.account_id = $2 \
               AND t.occurred_at >= $3 AND t.occurred_at < $4 \
             ORDER BY t.occurred_at ASC, t.id ASC",
        )
        .bind(user_id)
        .bind(aid)
        .bind(start)
        .bind(end)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as(
            "SELECT t.id, t.account_id, t.direction, t.amount_fen, t.category, t.merchant, t.memo, t.occurred_at, t.status \
             FROM transactions t JOIN accounts a ON a.id = t.account_id \
             WHERE a.user_id = $1 \
               AND t.occurred_at >= $2 AND t.occurred_at < $3 \
             ORDER BY t.occurred_at ASC, t.id ASC",
        )
        .bind(user_id)
        .bind(start)
        .bind(end)
        .fetch_all(pool)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_month_basic() {
        assert_eq!(parse_month("2026-10"), Some((2026, 10)));
        assert_eq!(parse_month("  2026-10  "), Some((2026, 10)));
        assert_eq!(parse_month("2026-10-15"), Some((2026, 10)));
        assert_eq!(parse_month("2026-13"), None);
        assert_eq!(parse_month("20-10"), None);
        assert_eq!(parse_month(""), None);
    }

    #[test]
    fn month_window_december_wraps() {
        let (s, e) = month_window(Some("2026-12"));
        assert_eq!(s.format("%Y-%m-%d").to_string(), "2026-12-01");
        assert_eq!(e.format("%Y-%m-%d").to_string(), "2027-01-01");
    }

    #[test]
    fn csv_field_escapes_commas_and_quotes() {
        assert_eq!(csv_field("a,b", ','), "\"a,b\"");
        assert_eq!(csv_field("a\"b", ','), "\"a\"\"b\"");
        assert_eq!(csv_field("a\tb", '\t'), "a\tb"); // tsv: tab-OK
    }
}
