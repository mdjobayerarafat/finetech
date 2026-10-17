//! Smart-transfer HTTP routes.
//!
//! `POST /api/v1/smart-transfers/parse`        — NL → structured draft
//! `POST /api/v1/smart-transfers/preview`      — security review + payee resolve
//! `POST /api/v1/smart-transfers/execute`      — immediate debit (Green) or
//!                                                pending_action (Yellow/Red)
//! `GET  /api/v1/smart-transfers/payees`       — list + search payees (with ?q=)
//! `POST /api/v1/smart-transfers/payees`       — add a payee
//! `GET  /api/v1/smart-transfers/beneficial-accounts` — list real beneficiary list
//! `POST /api/v1/smart-transfers/beneficial-accounts` — add a beneficiary
//! `POST /api/v1/smart-transfers/schedule`     — schedule a transfer
//! `GET  /api/v1/smart-transfers/scheduled`    — list scheduled transfers
//! `POST /api/v1/split-bills`                  — create split-bill
//! `GET  /api/v1/split-bills`                  — list split-bills
//! `POST /api/v1/split-bills/{id}/parts/{part_id}/pay` — mark part paid

use actix_web::{web, HttpResponse};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::audit::{self, AppendEventParams};
use crate::config;
use crate::ctx::{require_csrf, AuthUser, RequestId};
use crate::errors::{self, ApiError};
use crate::ids;
use crate::models::Payee;
use crate::planner;
use crate::safety;
use crate::smart_transfers;
use crate::state::AppState;
use crate::tools::{self};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/smart-transfers/parse", web::post().to(parse))
        .route("/smart-transfers/preview", web::post().to(preview))
        .route("/smart-transfers/execute", web::post().to(execute))
        .route("/smart-transfers/payees", web::get().to(list_payees))
        .route("/smart-transfers/payees", web::post().to(create_payee))
        .route("/smart-transfers/beneficial-accounts", web::get().to(list_beneficial_accounts))
        .route("/smart-transfers/beneficial-accounts", web::post().to(create_beneficial_account))
        .route("/smart-transfers/schedule", web::post().to(schedule))
        .route("/smart-transfers/scheduled", web::get().to(list_scheduled))
        .route("/split-bills", web::get().to(list_split_bills))
        .route("/split-bills", web::post().to(create_split_bill))
        .route(
            "/split-bills/{bill_id}/parts/{part_id}/pay",
            web::post().to(mark_part_paid),
        );
}

// --- Payee book ------------------------------------------------------------

#[derive(Deserialize)]
pub struct PayeeQuery {
    pub q: Option<String>,
}

async fn list_payees(
    state: web::Data<AppState>,
    auth: AuthUser,
    q: web::Query<PayeeQuery>,
) -> Result<HttpResponse, ApiError> {
    let user = auth.0;
    let q = q.into_inner().q.unwrap_or_default();
    if !q.is_empty() {
        let suggestions = smart_transfers::suggest_payees(&state.pool, &user, &q, 20)
            .await
            .map_err(|_| errors::internal_error())?;
        let out: Vec<_> = suggestions
            .into_iter()
            .map(|s| {
                json!({
                    "payee": s.payee,
                    "match_kind": s.match_kind,
                    "confidence": s.confidence,
                    "note": s.note,
                })
            })
            .collect();
        return Ok(HttpResponse::Ok().json(json!({
            "query": q,
            "suggestions": out,
        })));
    }
    let rows: Vec<Payee> = sqlx::query_as(
        "SELECT * FROM payees WHERE owner_user_id = $1 ORDER BY verified DESC, display_name ASC",
    )
    .bind(&user.id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    Ok(HttpResponse::Ok().json(rows))
}

#[derive(Deserialize)]
pub struct NewPayeeIn {
    pub display_name: String,
    pub phone_tail: Option<String>,
    pub note: Option<String>,
    #[serde(default = "default_destination")]
    pub mock_destination: String,
}
fn default_destination() -> String {
    "****demo".to_string()
}

async fn create_payee(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    body: web::Json<NewPayeeIn>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let user = auth.0;
    let body = body.into_inner();
    if body.display_name.trim().is_empty() {
        return Err(errors::schema_invalid("display_name is required"));
    }
    let id = ids::new_id("payee");
    let now = Utc::now();
    // The note is encoded into the `mock_destination` field for the
    // demo (the column was the only freeform text we could add without
    // a migration). Real banks would carry it as a separate field.
    let dest = if let Some(note) = body.note {
        if note.trim().is_empty() {
            body.mock_destination
        } else {
            format!("note: {}", note)
        }
    } else {
        body.mock_destination
    };
    sqlx::query(
        "INSERT INTO payees (id, owner_user_id, display_name, phone_tail, mock_destination, verified, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(&id)
    .bind(&user.id)
    .bind(&body.display_name)
    .bind(&body.phone_tail)
    .bind(&dest)
    .bind(false)
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(|e| {
        log::warn!("smart.payee.create insert: {e}");
        errors::internal_error()
    })?;
    audit::append_event(
        &state.pool,
        AppendEventParams {
            action: "smart.payee.create",
            actor_user_id: Some(&user.id),
            session: None,
            request_id: Some(&rid.0),
            workflow_id: None,
            node_id: None,
            tool: None,
            parameters_masked: Some(json!({
                "payee_id": id,
                "display_name": body.display_name,
                "has_phone": body.phone_tail.is_some(),
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
    let row: Payee = sqlx::query_as("SELECT * FROM payees WHERE id = $1")
        .bind(&id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    Ok(HttpResponse::Ok().json(row))
}

// --- Beneficial-account directory -----------------------------------------
//
// In real banking a "beneficial account" is the payee's account — the
// destination, as opposed to the remitter. The frontend Beneficial
// accounts page uses these routes to build and search a real, persistent
// directory the user owns. They're persisted in the same `payees` table
// as the regular smart-transfer list; the only difference is the
// expanded `mock_destination` field, which carries the bank-name hint
// for the UI and the full "bank · account tail · note" string for
// debugging. New payees added here are immediately resolvable by the
// smart-transfer preview/parse path.

#[derive(Deserialize)]
pub struct BeneficialQuery {
    /// Free-text search across display name, phone tail and bank hint.
    pub q: Option<String>,
}

async fn list_beneficial_accounts(
    state: web::Data<AppState>,
    auth: AuthUser,
    q: web::Query<BeneficialQuery>,
) -> Result<HttpResponse, ApiError> {
    let user = auth.0;
    let needle = q.into_inner().q.unwrap_or_default();
    let rows: Vec<Payee> = if needle.trim().is_empty() {
        sqlx::query_as(
            "SELECT * FROM payees WHERE owner_user_id = $1 \
             ORDER BY display_name ASC",
        )
        .bind(&user.id)
        .fetch_all(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?
    } else {
        // Case-insensitive prefix/contains match across three columns.
        let pat = format!("%{}%", needle);
        sqlx::query_as(
            "SELECT * FROM payees WHERE owner_user_id = $1 \
             AND (display_name ILIKE $2 OR phone_tail ILIKE $2 OR mock_destination ILIKE $2) \
             ORDER BY display_name ASC",
        )
        .bind(&user.id)
        .bind(&pat)
        .fetch_all(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?
    };
    let out: Vec<_> = rows
        .into_iter()
        .map(|p| {
            json!({
                "id": p.id,
                "display_name": p.display_name,
                "phone_tail": p.phone_tail,
                "bank_hint": extract_bank_hint(&p.mock_destination),
                "destination": p.mock_destination,
                "verified": p.verified,
                "first_used_at": p.first_used_at,
                "created_at": p.created_at,
            })
        })
        .collect();
    Ok(HttpResponse::Ok().json(out))
}

#[derive(Deserialize)]
pub struct NewBeneficialIn {
    pub display_name: String,
    pub phone_tail: Option<String>,
    /// Optional free-text bank hint (e.g. "ICBC", "招商银行 尾号 1234").
    /// Stored in `mock_destination` as `"bank:<hint>"` so it's queryable
    /// by the list endpoint and round-trips intact.
    pub bank_hint: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

async fn create_beneficial_account(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    body: web::Json<NewBeneficialIn>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let user = auth.0;
    let body = body.into_inner();
    if body.display_name.trim().is_empty() {
        return Err(errors::schema_invalid("display_name is required"));
    }
    let id = ids::new_id("payee");
    let now = Utc::now();
    // Compose the destination as a single human-readable string so
    // everything round-trips through the existing `payees.mock_destination`
    // column without a migration. Format: "bank:<hint> | note:<note>".
    let mut parts: Vec<String> = Vec::new();
    if let Some(b) = body.bank_hint.as_ref() {
        if !b.trim().is_empty() {
            parts.push(format!("bank:{}", b.trim()));
        }
    }
    if let Some(n) = body.note.as_ref() {
        if !n.trim().is_empty() {
            parts.push(format!("note:{}", n.trim()));
        }
    }
    let dest = if parts.is_empty() {
        "****demo".to_string()
    } else {
        parts.join(" | ")
    };
    sqlx::query(
        "INSERT INTO payees (id, owner_user_id, display_name, phone_tail, mock_destination, verified, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(&id)
    .bind(&user.id)
    .bind(&body.display_name.trim())
    .bind(body.phone_tail.as_ref().map(|s| s.trim().to_string()))
    .bind(&dest)
    .bind(false)
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(|e| {
        log::warn!("smart.beneficial.create insert: {e}");
        errors::internal_error()
    })?;
    audit::append_event(
        &state.pool,
        AppendEventParams {
            action: "beneficial.create",
            actor_user_id: Some(&user.id),
            session: None,
            request_id: Some(&rid.0),
            workflow_id: None,
            node_id: None,
            tool: None,
            parameters_masked: Some(json!({
                "payee_id": id,
                "display_name": body.display_name,
                "has_phone": body.phone_tail.is_some(),
                "has_bank": body.bank_hint.as_ref().map(|s| !s.is_empty()).unwrap_or(false),
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
    let row: Payee = sqlx::query_as("SELECT * FROM payees WHERE id = $1")
        .bind(&id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    Ok(HttpResponse::Ok().json(json!({
        "id": row.id,
        "display_name": row.display_name,
        "phone_tail": row.phone_tail,
        "bank_hint": extract_bank_hint(&row.mock_destination),
        "destination": row.mock_destination,
        "verified": row.verified,
        "first_used_at": row.first_used_at,
        "created_at": row.created_at,
    })))
}

/// Pull `bank:<hint>` out of a stored `mock_destination` string. Returns
/// `None` if there isn't one.
fn extract_bank_hint(dest: &str) -> Option<String> {
    for part in dest.split('|') {
        let p = part.trim();
        if let Some(rest) = p.strip_prefix("bank:") {
            return Some(rest.trim().to_string());
        }
    }
    None
}

// --- Parse + preview -------------------------------------------------------

#[derive(Deserialize)]
pub struct ParseIn {
    pub text: String,
    pub language: Option<String>,
}

async fn parse(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    body: web::Json<ParseIn>,
) -> Result<HttpResponse, ApiError> {
    let user = auth.0;
    let body = body.into_inner();
    if body.text.trim().is_empty() {
        return Err(errors::schema_invalid("text is required"));
    }
    let settings = config::get_settings();
    let draft = smart_transfers::parse_request(&state.pool, settings, &user, &body.text).await;
    audit::append_event(
        &state.pool,
        AppendEventParams {
            action: "smart.transfer.parse",
            actor_user_id: Some(&user.id),
            session: None,
            request_id: Some(&rid.0),
            workflow_id: None,
            node_id: None,
            tool: None,
            parameters_masked: Some(json!({
                "text_chars": body.text.chars().count(),
                "language": body.language,
                "needs_clarification": draft.get("needs_clarification").cloned().unwrap_or(Value::Null),
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
    Ok(HttpResponse::Ok().json(draft))
}

#[derive(Deserialize)]
pub struct PreviewIn {
    pub payee_id: Option<String>,
    pub payee_name: Option<String>,
    pub amount_fen: i64,
    pub memo: Option<String>,
    pub schedule_iso: Option<String>,
    pub account_id: Option<String>,
}

async fn preview(
    state: web::Data<AppState>,
    auth: AuthUser,
    body: web::Json<PreviewIn>,
) -> Result<HttpResponse, ApiError> {
    let user = auth.0;
    let body = body.into_inner();
    if body.amount_fen <= 0 {
        return Err(errors::schema_invalid("amount_fen must be > 0"));
    }
    // Resolve payee by id, then name, then by id-prefix / fuzzy match.
    let payee: Payee = if let Some(pid) = body.payee_id.as_deref() {
        sqlx::query_as::<_, Payee>("SELECT * FROM payees WHERE id = $1 AND owner_user_id = $2")
            .bind(pid)
            .bind(&user.id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|_| errors::internal_error())?
            .ok_or_else(|| errors::not_found("Payee not found."))?
    } else if let Some(name) = body.payee_name.as_deref() {
        // Try the smart resolver first; if it returns a single payee, use it.
        let resolved = smart_transfers::resolve(&state.pool, &user, name)
            .await
            .map_err(|_| errors::internal_error())?;
        match resolved.status.as_str() {
            "resolved" => resolved.payee.expect("resolve: payee missing"),
            _ => {
                return Err(errors::clarification_required(vec![format!(
                    "I couldn't find a unique payee for {:?}. Try adding a beneficial account first.",
                    name
                )]));
            }
        }
    } else {
        return Err(errors::schema_invalid("payee_id or payee_name is required"));
    };
    let schedule = body
        .schedule_iso
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&Utc));
    let memo = body.memo.as_deref().unwrap_or("");
    let settings = config::get_settings();
    let review = smart_transfers::security_review(
        &state.pool,
        settings,
        &user,
        &payee,
        body.amount_fen,
        memo,
        schedule,
    )
    .await;
    // Lightweight balance check on the selected account.
    let balance_check: Option<Value> = if let Some(aid) = body.account_id.as_deref() {
        let row: Option<(i64,)> = sqlx::query_as(
            "SELECT available_balance_fen FROM accounts WHERE id = $1 AND user_id = $2",
        )
        .bind(aid)
        .bind(&user.id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
        row.map(|(b,)| {
            json!({
                "account_id": aid,
                "available_balance_fen": b,
                "after_transfer_fen": b - body.amount_fen,
            })
        })
    } else {
        None
    };
    Ok(HttpResponse::Ok().json(json!({
        "payee": payee,
        "amount_fen": body.amount_fen,
        "memo": memo,
        "schedule_iso": body.schedule_iso,
        "review": review,
        "balance_check": balance_check,
    })))
}

// --- Schedule + list scheduled --------------------------------------------

#[derive(Deserialize)]
pub struct ScheduleIn {
    pub payee_id: String,
    pub amount_fen: i64,
    pub memo: Option<String>,
    pub schedule_iso: String,
    pub account_id: Option<String>,
}

// --- Execute immediate transfer -------------------------------------------
//
// `POST /api/v1/smart-transfers/execute` is the "money moves now" path.
// It's the pay-now counterpart of `schedule` and is wired up by the
// frontend Transfers page so the user sees the balance drop on success.
//
// Behaviour matrix:
//   - Green tier: debit immediately via the same lock + ledger path as
//     `t_transfer_create_pending` and return the `transfer_id`.
//   - Yellow / Red tier: return 403 with `pending_action_id` so the
//     existing `/actions/{id}/verify` (mock MFA for Red) + `/actions/{id}
//     /confirm` flow runs. Money does NOT move on this branch.
//
// The actual ledger math lives in `tools::t_transfer_create_pending` so
// schedule and execute can't drift apart.

#[derive(Deserialize)]
pub struct ExecuteIn {
    pub payee_id: String,
    pub amount_fen: i64,
    pub memo: Option<String>,
    pub account_id: Option<String>,
    /// Optional client-supplied idempotency key. We default to
    /// `idem_{user_id}_{payee_id}_{amount_fen}_{request_id}` so a back-
    /// button double-fire in the UI doesn't double-debit. Duplicate keys
    /// return the existing transfer row.
    pub idempotency_key: Option<String>,
}

async fn execute(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    body: web::Json<ExecuteIn>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    safety::assert_unlocked(&auth.0)?;
    let user = auth.0;
    let body = body.into_inner();
    if body.amount_fen <= 0 {
        return Err(errors::schema_invalid("amount_fen must be > 0"));
    }
    let payee: Payee = sqlx::query_as::<_, Payee>(
        "SELECT * FROM payees WHERE id = $1 AND owner_user_id = $2",
    )
    .bind(&body.payee_id)
    .bind(&user.id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?
    .ok_or_else(|| errors::not_found("Payee not found."))?;
    // Resolve the source account: explicit id, else the user's first one.
    let account_id: String = if let Some(aid) = body.account_id.as_deref() {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM accounts WHERE id = $1 AND user_id = $2",
        )
        .bind(aid)
        .bind(&user.id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
        match row {
            Some((id,)) => id,
            None => return Err(errors::not_found("Account not found.")),
        }
    } else {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM accounts WHERE user_id = $1 ORDER BY created_at ASC LIMIT 1",
        )
        .bind(&user.id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| errors::not_found("No account on file for this user."))?;
        match row {
            Some((id,)) => id,
            None => return Err(errors::not_found("No account on file for this user.")),
        }
    };
    let settings = config::get_settings();
    let review = smart_transfers::security_review(
        &state.pool,
        settings,
        &user,
        &payee,
        body.amount_fen,
        body.memo.as_deref().unwrap_or(""),
        None,
    )
    .await;
    let tier = review.recommended_tier.clone();
    if tier == "Yellow" || tier == "Red" {
        // Build a planner workflow so the standard pending_actions row +
        // parameter hash are produced. Then surface its first node's
        // pending_action_id back to the client. The frontend
        // ConfirmationCard already drives the verify + confirm loop.
        let action = json!({
            "tool": "transfer.create_pending",
            "arguments": {
                "account_id": account_id,
                "payee_id": payee.id,
                "amount_fen": body.amount_fen,
                "memo": body.memo.clone().unwrap_or_default(),
            },
        });
        let workflow = match planner::build_plan(&state.pool, &user, &[action], &rid.0).await {
            Ok(w) => w,
            Err(crate::tools::ToolError::NotFound(m)) => return Err(errors::not_found(&m)),
            Err(crate::tools::ToolError::InsufficientFunds(m)) => {
                return Err(errors::ApiError::new("insufficient_funds", &m, 422));
            }
            Err(crate::tools::ToolError::Clarification(m)) => {
                return Err(errors::clarification_required(vec![m]));
            }
            Err(e) => return Err(errors::schema_invalid(&e.to_string())),
        };
        let nodes: Vec<crate::models::WorkflowNode> = sqlx::query_as(
            "SELECT * FROM workflow_nodes WHERE workflow_id = $1 ORDER BY node_index ASC",
        )
        .bind(&workflow.id)
        .fetch_all(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
        let pa_id = nodes.first().and_then(|n| n.pending_action_id.clone());
        return Err(errors::ApiError::new(
            "approval_required",
            &format!("Transfer needs your {} confirmation before posting.", tier),
            403,
        )
        .with_details(json!({
            "pending_action_id": pa_id,
            "tier": tier,
            "review": review,
        })));
    }
    // Green — debit immediately.
    let idem = body.idempotency_key.unwrap_or_else(|| {
        format!("idem_{}_{}_{}_{}", user.id, payee.id, body.amount_fen, rid.0)
    });
    // Replay protection: a transfer with this idempotency_key + account
    // already exists → return the existing row, no double-debit.
    let existing: Option<(String, String, i64, Option<DateTime<Utc>>)> = sqlx::query_as(
        "SELECT id, status, amount_fen, posted_at FROM transfers \
         WHERE idempotency_key = $1 AND source_account_id = $2",
    )
    .bind(&idem)
    .bind(&account_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    if let Some((transfer_id, status, amount_fen, posted_at)) = existing {
        return Ok(HttpResponse::Ok().json(json!({
            "transfer_id": transfer_id,
            "status": status,
            "amount_fen": amount_fen,
            "posted_at": posted_at.map(|d| d.to_rfc3339()),
            "pending_action_id": Value::Null,
            "replay": true,
        })));
    }
    let args = json!({
        "account_id": account_id,
        "payee_id": payee.id,
        "amount_fen": body.amount_fen,
        "memo": body.memo.clone().unwrap_or_default(),
        "idempotency_key": idem,
    });
    let ctx = crate::tools::ToolContext {
        pool: state.pool.clone(),
        user: user.clone(),
        request_id: rid.0.clone(),
        session_id: None,
    };
    let result = crate::tools::t_transfer_create_pending(&ctx, &args).await;
    match result {
        Ok(v) => Ok(HttpResponse::Ok().json(json!({
            "transfer_id": v.get("transfer_id").cloned().unwrap_or(Value::Null),
            "status": v.get("status").cloned().unwrap_or(Value::String("posted".into())),
            "amount_fen": v.get("amount_fen").cloned().unwrap_or(Value::Number(0.into())),
            "posted_at": v.get("posted_at").cloned().unwrap_or(Value::Null),
            "pending_action_id": Value::Null,
            "review": review,
            "replay": false,
        }))),
        Err(crate::tools::ToolError::InsufficientFunds(m)) => {
            Err(errors::ApiError::new("insufficient_funds", &m, 422))
        }
        Err(crate::tools::ToolError::Schema(m)) => Err(errors::schema_invalid(&m)),
        Err(e) => Err(errors::schema_invalid(&e.to_string())),
    }
}

async fn schedule(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    body: web::Json<ScheduleIn>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let user = auth.0;
    let body = body.into_inner();
    if body.amount_fen <= 0 {
        return Err(errors::schema_invalid("amount_fen must be > 0"));
    }
    let schedule_dt: DateTime<Utc> = body
        .schedule_iso
        .parse::<DateTime<Utc>>()
        .map_err(|_| errors::schema_invalid("schedule_iso must be ISO-8601"))?;
    if schedule_dt <= Utc::now() {
        return Err(errors::schema_invalid("schedule_iso must be in the future"));
    }
    // Confirm the payee exists and belongs to this user.
    let payee: Payee = sqlx::query_as::<_, Payee>(
        "SELECT * FROM payees WHERE id = $1 AND owner_user_id = $2",
    )
    .bind(&body.payee_id)
    .bind(&user.id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?
    .ok_or_else(|| errors::not_found("Payee not found."))?;
    // Build a transfer row directly in `scheduled` state. The actual
    // posting is handled by a future job (out of scope for the demo).
    let xfer_id = ids::new_id("xfer");
    let now = Utc::now();
    // Resolve a real account: explicit id, else the user's first one.
    let account_id: String = if let Some(aid) = body.account_id.as_deref() {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM accounts WHERE id = $1 AND user_id = $2",
        )
        .bind(aid)
        .bind(&user.id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
        match row {
            Some((id,)) => id,
            None => return Err(errors::not_found("Account not found.")),
        }
    } else {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM accounts WHERE user_id = $1 ORDER BY created_at ASC LIMIT 1",
        )
        .bind(&user.id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| errors::not_found("No account on file for this user."))?;
        match row {
            Some((id,)) => id,
            None => return Err(errors::not_found("No account on file for this user.")),
        }
    };
    sqlx::query(
        "INSERT INTO transfers (id, source_account_id, payee_id, amount_fen, memo, schedule, idempotency_key, status, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, 'scheduled', $8)",
    )
    .bind(&xfer_id)
    .bind(&account_id)
    .bind(&payee.id)
    .bind(body.amount_fen)
    .bind(body.memo.as_deref().unwrap_or(""))
    .bind(schedule_dt)
    .bind(format!("sched_{}", xfer_id))
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(|e| {
        log::warn!("smart.transfer.schedule insert: {e}");
        errors::internal_error()
    })?;
    audit::append_event(
        &state.pool,
        AppendEventParams {
            action: "smart.transfer.schedule",
            actor_user_id: Some(&user.id),
            session: None,
            request_id: Some(&rid.0),
            workflow_id: None,
            node_id: None,
            tool: None,
            parameters_masked: Some(json!({
                "transfer_id": xfer_id,
                "payee_id": payee.id,
                "amount_fen": body.amount_fen,
                "schedule_iso": body.schedule_iso,
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
    Ok(HttpResponse::Ok().json(json!({
        "transfer_id": xfer_id,
        "payee_id": payee.id,
        "amount_fen": body.amount_fen,
        "schedule_iso": schedule_dt.to_rfc3339(),
        "status": "scheduled",
    })))
}

#[derive(Deserialize, Default)]
pub struct ScheduledQuery {
    pub status: Option<String>,
}

async fn list_scheduled(
    state: web::Data<AppState>,
    auth: AuthUser,
    q: web::Query<ScheduledQuery>,
) -> Result<HttpResponse, ApiError> {
    let user = auth.0;
    let status = q.into_inner().status.unwrap_or_else(|| "scheduled".into());
    let rows: Vec<(String, String, i64, String, Option<DateTime<Utc>>, String, DateTime<Utc>)> =
        sqlx::query_as(
            "SELECT t.id, t.payee_id, t.amount_fen, t.memo, t.schedule, t.status, t.created_at \
             FROM transfers t JOIN accounts a ON a.id = t.source_account_id \
             WHERE a.user_id = $1 AND t.status = $2 \
             ORDER BY t.schedule ASC NULLS LAST",
        )
        .bind(&user.id)
        .bind(&status)
        .fetch_all(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    let out: Vec<_> = rows
        .into_iter()
        .map(|(id, payee_id, amount_fen, memo, schedule, status, created_at)| {
            json!({
                "id": id,
                "payee_id": payee_id,
                "amount_fen": amount_fen,
                "memo": memo,
                "schedule": schedule,
                "status": status,
                "created_at": created_at,
            })
        })
        .collect();
    Ok(HttpResponse::Ok().json(out))
}

// --- Split-bills -----------------------------------------------------------

#[derive(Deserialize)]
pub struct NewPartIn {
    pub payee_id: Option<String>,
    pub participant_name: String,
    pub participant_phone_tail: Option<String>,
    pub share_fen: i64,
}

#[derive(Deserialize)]
pub struct NewSplitBillIn {
    pub title: String,
    pub memo: Option<String>,
    pub total_fen: i64,
    pub currency: Option<String>,
    pub source_account_id: Option<String>,
    pub parts: Vec<NewPartIn>,
}

async fn create_split_bill(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    body: web::Json<NewSplitBillIn>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let user = auth.0;
    let body = body.into_inner();
    if body.total_fen <= 0 {
        return Err(errors::schema_invalid("total_fen must be > 0"));
    }
    if body.parts.is_empty() {
        return Err(errors::schema_invalid("parts must not be empty"));
    }
    let parts: Vec<smart_transfers::NewPart> = body
        .parts
        .into_iter()
        .map(|p| smart_transfers::NewPart {
            payee_id: p.payee_id,
            participant_name: p.participant_name,
            participant_phone_tail: p.participant_phone_tail,
            share_fen: p.share_fen,
        })
        .collect();
    let sum: i64 = parts.iter().map(|p| p.share_fen).sum();
    if sum != body.total_fen {
        return Err(errors::schema_invalid(&format!(
            "parts sum to {} fen, but total is {} fen",
            sum, body.total_fen
        )));
    }
    let currency = body.currency.unwrap_or_else(|| "CNY".into());
    let bill = smart_transfers::create_split_bill(
        &state.pool,
        &user,
        &body.title,
        body.memo.as_deref().unwrap_or(""),
        body.total_fen,
        &currency,
        body.source_account_id.as_deref(),
        None,
        parts,
    )
    .await
    .map_err(|e| {
        log::warn!("split bill create: {e}");
        errors::internal_error()
    })?;
    audit::append_event(
        &state.pool,
        AppendEventParams {
            action: "split_bill.create",
            actor_user_id: Some(&user.id),
            session: None,
            request_id: Some(&rid.0),
            workflow_id: None,
            node_id: None,
            tool: None,
            parameters_masked: Some(json!({
                "split_bill_id": bill.id,
                "total_fen": bill.total_fen,
                "parts": bill.total_fen, // share count not stored on bill; left as a denormalised hint
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
    Ok(HttpResponse::Ok().json(bill))
}

async fn list_split_bills(
    state: web::Data<AppState>,
    auth: AuthUser,
) -> Result<HttpResponse, ApiError> {
    let user = auth.0;
    let bills = smart_transfers::list_split_bills(&state.pool, &user)
        .await
        .map_err(|_| errors::internal_error())?;
    Ok(HttpResponse::Ok().json(bills))
}

#[derive(Deserialize)]
pub struct MarkPaidIn {
    pub transfer_id: String,
}

async fn mark_part_paid(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    path: web::Path<(String, String)>,
    body: web::Json<MarkPaidIn>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let user = auth.0;
    let (_bill_id, part_id) = path.into_inner();
    let body = body.into_inner();
    let updated = smart_transfers::mark_part_paid(&state.pool, &user, &part_id, &body.transfer_id)
        .await
        .map_err(|_| errors::not_found("Part not found."))?;
    audit::append_event(
        &state.pool,
        AppendEventParams {
            action: "split_bill.part.paid",
            actor_user_id: Some(&user.id),
            session: None,
            request_id: Some(&rid.0),
            workflow_id: None,
            node_id: None,
            tool: None,
            parameters_masked: Some(json!({
                "split_bill_id": updated.split_bill_id,
                "part_id": updated.id,
                "transfer_id": body.transfer_id,
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
    Ok(HttpResponse::Ok().json(updated))
}
