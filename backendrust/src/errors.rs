//! Typed API errors with the same envelope as the Python backend.
//!
//! All errors are returned as:
//! ```json
//! { "error": { "code": "...", "message": "...", "trace_id": "..." } }
//! ```

use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use serde::Serialize;
use serde_json::Value;
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct ApiErrorBody {
    pub code: String,
    pub message: String,
    pub trace_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub http_status: u16,
    pub details: Option<Value>,
    pub trace_id: String,
}

impl ApiError {
    pub fn new(code: &str, message: &str, http_status: u16) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
            http_status,
            details: None,
            trace_id: Uuid::new_v4().simple().to_string(),
        }
    }

    pub fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn with_trace(mut self, trace_id: &str) -> Self {
        self.trace_id = trace_id.to_string();
        self
    }

    pub fn body(&self) -> ApiErrorBody {
        ApiErrorBody {
            code: self.code.clone(),
            message: self.message.clone(),
            trace_id: self.trace_id.clone(),
            details: self.details.clone(),
        }
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ApiError {}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        StatusCode::from_u16(self.http_status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
    }

    fn error_response(&self) -> HttpResponse {
        let body = serde_json::json!({ "error": self.body() });
        HttpResponse::build(self.status_code()).json(body)
    }
}

// --- Convenience constructors ------------------------------------------

pub fn schema_invalid(message: &str) -> ApiError {
    ApiError::new("schema_invalid", message, 400)
}

pub fn bad_request(message: &str) -> ApiError {
    ApiError::new("bad_request", message, 400)
}

pub fn schema_invalid_details<T: Serialize>(message: &str, details: T) -> ApiError {
    let v = serde_json::to_value(details).unwrap_or(Value::Null);
    ApiError::new("schema_invalid", message, 400).with_details(v)
}

pub fn clarification_required(questions: Vec<String>) -> ApiError {
    ApiError::new(
        "clarification_required",
        "The agent needs more information to continue.",
        400,
    )
    .with_details(serde_json::json!({ "questions": questions }))
}

pub fn unauthenticated(message: &str) -> ApiError {
    ApiError::new("unauthenticated", message, 401)
}

pub fn forbidden(message: &str) -> ApiError {
    ApiError::new("forbidden", message, 403)
}

pub fn csrf_invalid() -> ApiError {
    ApiError::new(
        "csrf_invalid",
        "CSRF token is missing or invalid.",
        403,
    )
}

pub fn policy_denied(reason: &str) -> ApiError {
    ApiError::new("policy_denied", "Policy does not allow this action.", 403)
        .with_details(serde_json::json!({ "reason": reason }))
}

pub fn safety_locked(reason: Option<&str>) -> ApiError {
    let mut e = ApiError::new(
        "safety_locked",
        "Account is temporarily locked. Please contact support.",
        403,
    );
    if let Some(r) = reason {
        e = e.with_details(serde_json::json!({ "reason": r }));
    }
    e
}

pub fn not_found(message: &str) -> ApiError {
    ApiError::new("not_found", message, 404)
}

pub fn parameter_changed() -> ApiError {
    ApiError::new(
        "parameter_changed",
        "Approval parameters have changed since confirmation.",
        409,
    )
}

pub fn idempotency_replay() -> ApiError {
    ApiError::new(
        "idempotency_replay",
        "This idempotency key was used with different parameters.",
        409,
    )
}

pub fn expired() -> ApiError {
    ApiError::new("expired", "This approval has expired.", 410)
}

pub fn daily_limit_exceeded(limit_fen: i64, attempted_total_fen: i64) -> ApiError {
    ApiError::new(
        "daily_limit_exceeded",
        "This transfer would exceed your daily limit.",
        422,
    )
    .with_details(serde_json::json!({
        "limit_fen": limit_fen,
        "attempted_total_fen": attempted_total_fen,
    }))
}

pub fn suitability_failed(reason: &str) -> ApiError {
    ApiError::new(
        "suitability_failed",
        "This product is not suitable for your profile.",
        422,
    )
    .with_details(serde_json::json!({ "reason": reason }))
}

pub fn internal_error() -> ApiError {
    ApiError::new("internal_error", "An internal error occurred.", 500)
}
