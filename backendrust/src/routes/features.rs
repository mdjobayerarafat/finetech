//! New demo features: red-team, forecast, receipt scan, voice.

use actix_web::{web, HttpResponse};
use base64::Engine;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::audit::{self, AppendEventParams};
use crate::config;
use crate::ctx::{require_csrf, AuthUser, RequestId};
use crate::errors::{self, ApiError};
use crate::forecast;
use crate::fraud;
use crate::models::Payee;
use crate::policy;
use crate::receipt;
use crate::redteam;
use crate::state::AppState;
use crate::voice;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/redteam/samples", web::get().to(redteam_samples))
        .route("/redteam/test", web::post().to(redteam_test))
        .route("/forecast/simulate", web::post().to(forecast_simulate))
        .route("/receipts/scan", web::post().to(receipt_scan))
        .route("/voice/stt", web::post().to(voice_stt))
        .route("/voice/tts", web::post().to(voice_tts))
        .route("/fraud/score", web::post().to(fraud_score))
        .route("/fraud/narrate", web::post().to(fraud_narrate));
}

// --- Red team ---------------------------------------------------------------

async fn redteam_samples(auth: AuthUser) -> Result<HttpResponse, ApiError> {
    // The sample list is admin-only too — the list itself is part of
    // the security work product.
    let _admin = crate::ctx::require_admin(&auth).await?;
    Ok(HttpResponse::Ok().json(json!({ "samples": redteam::samples() })))
}

#[derive(Deserialize)]
pub struct RedTeamIn {
    pub text: String,
    pub where_: String,
    #[serde(default)]
    pub persist: bool,
}

async fn redteam_test(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    body: web::Json<RedTeamIn>,
) -> Result<HttpResponse, ApiError> {
    // The red-team panel is admin-only. It exists to *test* the security
    // layer; if a customer could run arbitrary probes against their own
    // session they'd be able to spam the audit log with safety events and
    // observe the sanitised view of injection patterns (which is the very
    // thing the panel is meant to keep private). Only admins may run it.
    let _admin = crate::ctx::require_admin(&auth).await?;
    require_csrf(&req, &auth).await?;
    let body = body.into_inner();
    let where_ = if body.where_.is_empty() { "memo".to_string() } else { body.where_ };
    let hit = redteam::run_test(&state.pool, &auth, &rid, &where_, &body.text, body.persist).await?;
    Ok(HttpResponse::Ok().json(hit))
}

// --- Forecast ---------------------------------------------------------------

#[derive(Deserialize)]
pub struct ForecastIn {
    pub amount_cny: f64,
    /// 0-based month index from "now" — when in the forecast window the
    /// spend happens. Defaults to 0 (next month).
    #[serde(default)]
    pub spend_month_index: i32,
    #[serde(default = "default_months")]
    pub months: i32,
}
fn default_months() -> i32 {
    12
}

async fn forecast_simulate(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    body: web::Json<ForecastIn>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let user = auth.0;
    let body = body.into_inner();
    let amount_fen = (body.amount_cny * 100.0).round() as i64;
    if amount_fen <= 0 {
        return Err(errors::schema_invalid("amount_cny must be positive"));
    }
    let settings = config::get_settings();
    let out = forecast::run(&state.pool, settings, &user, amount_fen, body.spend_month_index, body.months)
        .await
        .map_err(|_| errors::internal_error())?;
    audit::append_event(
        &state.pool,
        AppendEventParams {
            action: "forecast.simulate",
            actor_user_id: Some(&user.id),
            session: None,
            request_id: Some(&rid.0),
            workflow_id: None,
            node_id: None,
            tool: None,
            parameters_masked: Some(json!({
                "amount_cny": body.amount_cny,
                "months": body.months,
                "spend_month_index": body.spend_month_index,
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
    Ok(HttpResponse::Ok().json(out))
}

// --- Receipt scan -----------------------------------------------------------

#[derive(Deserialize)]
pub struct ReceiptIn {
    /// Optional declared merchant hint from the UI.
    pub merchant_hint: Option<String>,
    /// Optional declared amount in CNY.
    pub amount_cny: Option<f64>,
    /// Optional filename (when the user uploads a file). Used as a
    /// deterministic seed for the stub.
    pub filename: Option<String>,
    /// Optional base64 image data — we accept it but do not actually
    /// decode in the simulation. Length is recorded for observability.
    pub image_b64: Option<String>,
}

async fn receipt_scan(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    body: web::Json<ReceiptIn>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let body = body.into_inner();
    let bytes_len = body
        .image_b64
        .as_ref()
        .map(|s| base64::engine::general_purpose::STANDARD.decode(s).map(|v| v.len()).unwrap_or(0))
        .unwrap_or(0);
    let draft = receipt::scan(
        &state,
        &auth,
        &rid,
        body.filename,
        body.merchant_hint,
        body.amount_cny,
        bytes_len,
    )
    .await?;
    Ok(HttpResponse::Ok().json(draft))
}

// --- Voice ------------------------------------------------------------------

#[derive(Deserialize)]
pub struct VoiceSttIn {
    /// Base64-encoded audio bytes.
    pub audio_b64: String,
    pub mime: Option<String>,
}

async fn voice_stt(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    body: web::Json<VoiceSttIn>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let body = body.into_inner();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&body.audio_b64)
        .map_err(|_| errors::schema_invalid("audio_b64 is not valid base64"))?;
    let mime = body.mime.as_deref().unwrap_or("audio/webm");
    let settings = config::get_settings();
    let stt = match voice::llm_transcribe(settings, bytes.len(), mime).await {
        Some(s) => s,
        None => voice::build_stt_out(bytes.len(), mime),
    };
    // Audit; do NOT log the audio bytes.
    audit::append_event(
        &state.pool,
        AppendEventParams {
            action: "voice.stt",
            actor_user_id: Some(&auth.0.id),
            session: None,
            request_id: Some(&rid.0),
            workflow_id: None,
            node_id: None,
            tool: None,
            parameters_masked: Some(json!({
                "audio_bytes": bytes.len(),
                "mime": mime,
                "language": stt.language,
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
    Ok(HttpResponse::Ok().json(stt))
}

#[derive(Deserialize)]
pub struct VoiceTtsIn {
    pub text: String,
    pub language: Option<String>,
}

async fn voice_tts(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    body: web::Json<VoiceTtsIn>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let body = body.into_inner();
    if body.text.is_empty() {
        return Err(errors::schema_invalid("text is required"));
    }
    let tts = voice::build_tts_out(&body.text);
    audit::append_event(
        &state.pool,
        AppendEventParams {
            action: "voice.tts",
            actor_user_id: Some(&auth.0.id),
            session: None,
            request_id: Some(&rid.0),
            workflow_id: None,
            node_id: None,
            tool: None,
            parameters_masked: Some(json!({
                "text_chars": body.text.chars().count(),
                "language": body.language,
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
    Ok(HttpResponse::Ok().json(tts))
}

// --- Fraud guard score ------------------------------------------------------

#[derive(Deserialize)]
pub struct FraudScoreIn {
    pub payee_id: Option<String>,
    pub payee_name: Option<String>,
    pub amount_fen: i64,
    pub memo: Option<String>,
    /// When true, runs the LLM narration pass as well.
    #[serde(default)]
    pub with_narrative: bool,
}

async fn fraud_score(
    state: web::Data<AppState>,
    auth: AuthUser,
    body: web::Json<FraudScoreIn>,
) -> Result<HttpResponse, ApiError> {
    let body = body.into_inner();
    let user = auth.0;
    let memo = body.memo.as_deref().unwrap_or("");
    // Resolve payee: prefer explicit id, else pick first payee with
    // matching display name, else synthesise a transient one.
    let payee: Payee = if let Some(pid) = body.payee_id.as_deref() {
        sqlx::query_as::<_, Payee>("SELECT * FROM payees WHERE id = $1 AND owner_user_id = $2")
            .bind(pid)
            .bind(&user.id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|_| errors::internal_error())?
            .ok_or_else(|| errors::not_found("Payee not found."))?
    } else if let Some(name) = body.payee_name.as_deref() {
        sqlx::query_as::<_, Payee>(
            "SELECT * FROM payees WHERE owner_user_id = $1 AND LOWER(display_name) LIKE LOWER($2) ORDER BY created_at ASC LIMIT 1",
        )
        .bind(&user.id)
        .bind(format!("%{}%", name))
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?
        .unwrap_or_else(|| Payee {
            id: "transient".to_string(),
            owner_user_id: user.id.clone(),
            display_name: name.to_string(),
            phone_tail: None,
            mock_destination: "transient".to_string(),
            verified: false,
            first_used_at: None,
            created_at: chrono::Utc::now(),
        })
    } else {
        return Err(errors::schema_invalid("payee_id or payee_name is required"));
    };
    if body.amount_fen <= 0 {
        return Err(errors::schema_invalid("amount_fen must be positive"));
    }
    let mut assessment = fraud::score_transfer(&state.pool, &user, &payee, body.amount_fen, memo)
        .await
        .map_err(|_| errors::internal_error())?;
    if body.with_narrative {
        let settings = config::get_settings();
        if let Some(n) = fraud::narrate_scam_flags(settings, &assessment, &payee.display_name, body.amount_fen).await {
            assessment.narrative = Some(n);
        }
    }
    // Also surface the existing policy decision so the UI can show both.
    let policy_decision = policy::detect_injection(memo);
    Ok(HttpResponse::Ok().json(json!({
        "assessment": assessment,
        "payee": {
            "id": payee.id,
            "display_name": payee.display_name,
            "verified": payee.verified,
            "first_used_at": payee.first_used_at,
        },
        "policy_injection_phrase": policy_decision,
    })))
}

#[derive(Deserialize)]
pub struct FraudNarrateIn {
    pub flags: Vec<Value>,
    pub score: u8,
    pub payee_name: String,
    pub amount_fen: i64,
}

async fn fraud_narrate(
    _state: web::Data<AppState>,
    _auth: AuthUser,
    body: web::Json<FraudNarrateIn>,
) -> Result<HttpResponse, ApiError> {
    let body = body.into_inner();
    let assessment = fraud::FraudAssessment {
        flags: body
            .flags
            .into_iter()
            .filter_map(|v| {
                Some(fraud::FraudFlag {
                    code: v.get("code")?.as_str()?.to_string(),
                    message: v.get("message")?.as_str()?.to_string(),
                    weight: v.get("weight").and_then(|x| x.as_u64()).unwrap_or(1) as u8,
                })
            })
            .collect(),
        score: body.score,
        templated: String::new(),
        narrative: None,
    };
    let settings = config::get_settings();
    let narrative = fraud::narrate_scam_flags(settings, &assessment, &body.payee_name, body.amount_fen)
        .await
        .unwrap_or_else(|| assessment.templated.clone());
    Ok(HttpResponse::Ok().json(json!({ "narrative": narrative })))
}
