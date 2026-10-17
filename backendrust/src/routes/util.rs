//! Miscellaneous utility endpoints.
//!
//! Currently exposes:
//! - `POST /api/v1/util/translate` — synchronous, on-demand translation
//!   of a list of short strings. The route is intentionally thin: all
//!   the work lives in `crate::translate::translate_strings`, which is
//!   already best-effort / never-block. We add auth + a single CSRF
//!   check here and nothing else.

use actix_web::{web, HttpRequest, HttpResponse};
use serde::Deserialize;
use serde_json::json;

use crate::config;
use crate::ctx::{require_csrf, AuthUser};
use crate::errors::ApiError;
use crate::translate;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/util/translate", web::post().to(translate_endpoint));
}

#[derive(Deserialize)]
pub struct TranslateIn {
    /// The strings to translate. The route returns one entry per input,
    /// in the same order, on a best-effort basis.
    pub texts: Vec<String>,
    /// `"en"` or `"zh"`. Case-insensitive. Anything else falls through
    /// to the default heuristic in `translate::translate_strings`.
    pub target: String,
}

/// POST /api/v1/util/translate
///
/// Request:  `{ "texts": ["hello", "world"], "target": "zh" }`
/// Response: `{ "translations": ["你好", "世界"] }`
///
/// On any translation failure (model slow, no network, parse error)
/// the server returns the originals as the translations. This is
/// deliberate: the contract of the route is "give me a best-effort
/// translation; never fail the request".
async fn translate_endpoint(
    req: HttpRequest,
    auth: AuthUser,
    body: web::Json<TranslateIn>,
) -> Result<HttpResponse, ApiError> {
    // CSRF: a same-session user can read translations cheaply, but
    // POST still requires the header+cookie+session-row triple to
    // match. This is the same guard every other mutating route uses.
    require_csrf(&req, &auth).await?;
    let body = body.into_inner();
    let settings = config::get_settings();
    let translations = translate::translate_strings(&settings, &body.texts, &body.target).await;
    Ok(HttpResponse::Ok().json(json!({
        "translations": translations,
    })))
}
