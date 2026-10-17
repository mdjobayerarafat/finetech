//! Best-effort translation fallback.
//!
//! Contract: this module is **never** allowed to block or break the chat.
//! Every public entry point must be infallible from the caller's
//! perspective — any failure (network error, non-2xx, parse error, count
//! mismatch, slow model) collapses to "return the originals unchanged".
//! The chat handler wraps the call in a `tokio::time::timeout` so a slow
//! model cannot stall the response either.
//!
//! The translate endpoint is intentionally thin: it does no
//! authorisation, no caching, no rate limiting beyond the upstream
//! model. It's a single LLM round-trip with a tight budget.

use log::debug;
use serde_json::{json, Value};
use std::time::Duration;

use crate::config::Settings;
use crate::llm_http;

/// System prompt that asks the model to translate a numbered list of
/// strings. Kept very short on purpose — translation is a degenerate
/// task, and a long prompt burns tokens we don't have a budget for.
const SYSTEM_PROMPT: &str = "You are a translation engine. Translate each numbered input string into the target language. Preserve placeholders, JSON keys, monetary values, code, names, and URLs. Output only the translated strings, one per line, each prefixed with `<n>: ` where n is the 1-based index.";

/// Tight budget on a single translation call. A 4s wait is already
/// uncomfortably long for a chat round-trip — if the model is slower
/// than this we time out and return the originals. The chat handler
/// wraps us in its own `tokio::time::timeout` too, so this is the
/// second line of defence.
const TIMEOUT: Duration = Duration::from_secs(4);

/// Best-effort translation of a list of short strings into `target`.
///
/// Returns one entry per input element, in the same order. If the
/// target is empty, or if every input is already in the target
/// language, no HTTP call is made and the inputs are returned as-is.
/// On any error (timeout, non-2xx, parse failure, count mismatch) the
/// originals are returned unchanged. Never panics, never blocks the
/// caller beyond `TIMEOUT`.
pub async fn translate_strings(settings: &Settings, texts: &[String], target: &str) -> Vec<String> {
    // Always return the inputs verbatim when the request is degenerate.
    if texts.is_empty() {
        return texts.to_vec();
    }
    let target_norm = normalise_target(target);
    if target_norm.is_empty() {
        return texts.to_vec();
    }

    // Skip the call when every input is already in the target language.
    if texts.iter().all(|t| already_in_target(t, &target_norm)) {
        return texts.to_vec();
    }

    // Bail out fast if we have no endpoint to hit.
    if !settings.unsloth_enabled() {
        return texts.to_vec();
    }

    let inputs: Vec<String> = texts.to_vec();
    let target_owned = target_norm.clone();
    let fut = async move { call_model(settings, &inputs, &target_owned).await };
    match tokio::time::timeout(TIMEOUT, fut).await {
        Ok(Some(out)) => out,
        // Either timed out, or the inner call returned `None` (any
        // error path). Either way: fall back to the originals.
        Ok(None) | Err(_) => texts.to_vec(),
    }
}

/// Normalise the user-supplied target language to one of `"en"` / `"zh"`.
/// Anything else is treated as English and causes an early-out.
fn normalise_target(target: &str) -> String {
    let t = target.trim().to_lowercase();
    match t.as_str() {
        "zh" | "zh-cn" | "zh-hans" | "chinese" | "中文" => "zh".to_string(),
        "en" | "english" => "en".to_string(),
        _ => String::new(),
    }
}

/// Cheap heuristic: is `s` already plausibly in `target`?
///
/// For English we accept the string if every character is ASCII and
/// there's at least one letter (so we don't burn a round-trip on a
/// pure number or a single punctuation mark).
///
/// For Chinese we accept the string if it contains any CJK Unified
/// Ideograph — the LLM is unlikely to add value to a string that
/// already reads as Chinese.
fn already_in_target(s: &str, target: &str) -> bool {
    match target {
        "en" => {
            s.chars().any(|c| c.is_ascii_alphabetic())
                && s.chars().all(|c| c.is_ascii() || c.is_whitespace())
        }
        "zh" => s.chars().any(|c| matches!(c, '\u{4e00}'..='\u{9fff}')),
        _ => false,
    }
}

/// Build the OpenAI-compatible request, fire it, parse the response.
/// Returns `None` on any failure (network, HTTP, parse, count mismatch).
async fn call_model(settings: &Settings, texts: &[String], target: &str) -> Option<Vec<String>> {
    let base = settings.unsloth_url.trim_end_matches('/');
    let url = format!("{}/chat/completions", base);

    // User-side payload: a numbered list. We rely on the system prompt
    // telling the model to keep the same `<n>: ...` shape on the way
    // out so we can parse it back deterministically.
    let mut user_payload = String::new();
    user_payload.push_str(&format!("target: {}\n", target));
    for (i, t) in texts.iter().enumerate() {
        user_payload.push_str(&format!("{}: {}\n", i + 1, t));
    }

    let body = json!({
        "model": settings.unsloth_model,
        "messages": [
            { "role": "system", "content": SYSTEM_PROMPT },
            { "role": "user",   "content": user_payload },
        ],
        "temperature": 0.0,
        "max_tokens": 1024,
        "stream": false,
    });

    debug!("translate: POST {} ({} inputs)", url, texts.len());
    let resp = match llm_http::client()
        .post(&url)
        .bearer_auth(&settings.unsloth_api_key)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return None,
    };
    let status = resp.status();
    let text = match resp.text().await {
        Ok(t) => t,
        Err(_) => return None,
    };
    if !status.is_success() {
        return None;
    }
    let parsed: Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(_) => return None,
    };
    // OpenAI shape: { choices: [ { message: { content: "..." } } ] }
    let content = parsed
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|v| v.as_str())?;
    parse_indexed_lines(content, texts.len())
}

/// Parse the model's response into one string per input. The model is
/// expected to emit lines of the form `<n>: <text>`. We accept any
/// line that starts with a 1-based index followed by `:`. Lines that
/// don't match the shape are ignored, and any indices that are missing
/// after the parse is finished cause the whole call to fail (so the
/// caller falls back to the originals rather than surfacing partial
/// output).
fn parse_indexed_lines(content: &str, expected: usize) -> Option<Vec<String>> {
    if expected == 0 {
        return Some(Vec::new());
    }
    let mut out: Vec<Option<String>> = vec![None; expected];
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Skip markdown code-fence lines if the model emitted them.
        if trimmed.starts_with("```") {
            continue;
        }
        // Find the first `:` and split.
        let (head, rest) = match trimmed.find(':') {
            Some(i) => (&trimmed[..i], trimmed[i + 1..].trim()),
            None => continue,
        };
        let n: usize = match head.trim().parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        if n == 0 || n > expected {
            continue;
        }
        // First write wins. Models occasionally re-emit an index; we
        // take the first one rather than concatenating.
        if out[n - 1].is_none() {
            out[n - 1] = Some(rest.to_string());
        }
    }
    // Require every slot to be filled. A partial result is worse than
    // no result — the chat handler will fall back to the originals
    // and the user will see the same strings the LLM originally
    // emitted (which is at least consistent with the rest of the
    // response).
    let mut filled: Vec<String> = Vec::with_capacity(expected);
    for slot in out {
        match slot {
            Some(s) => filled.push(s),
            None => return None,
        }
    }
    Some(filled)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Settings;

    fn empty_settings() -> Settings {
        // Build a Settings with only the unsloth fields populated; the
        // rest is irrelevant to the parser.
        Settings {
            host: "0.0.0.0".into(),
            port: 8080,
            app_env: "test".into(),
            database_url: String::new(),
            session_secret: String::new(),
            session_ttl_seconds: 0,
            pending_action_ttl_seconds: 0,
            cookie_secure: false,
            openai_api_key: String::new(),
            openai_base_url: String::new(),
            openai_model: String::new(),
            openrouter_referer: String::new(),
            openrouter_title: String::new(),
            unsloth_url: std::env::var("TEST_UNSLOTH_URL").unwrap_or_default(),
            unsloth_api_key: std::env::var("TEST_UNSLOTH_KEY").unwrap_or_default(),
            unsloth_model: "test-model".into(),
            daily_limit_fen: 0,
            mfa_demo_code: String::new(),
            max_failed_mfa: 0,
        }
    }

    #[test]
    fn parse_indexed_lines_basic() {
        let body = "1: 你好\n2: 世界\n";
        let out = parse_indexed_lines(body, 2).unwrap();
        assert_eq!(out, vec!["你好".to_string(), "世界".to_string()]);
    }

    #[test]
    fn parse_indexed_lines_ignores_fences_and_blank() {
        let body = "```\n1: alpha\n\n2: beta\n```\n";
        let out = parse_indexed_lines(body, 2).unwrap();
        assert_eq!(out, vec!["alpha".to_string(), "beta".to_string()]);
    }

    #[test]
    fn parse_indexed_lines_missing_index_fails() {
        // Only 1 of 2 was emitted — we treat the call as failed so the
        // caller falls back to the originals.
        let body = "1: only one\n";
        assert!(parse_indexed_lines(body, 2).is_none());
    }

    #[test]
    fn parse_indexed_lines_out_of_range_ignored() {
        let body = "1: ok\n3: out-of-range\n";
        let out = parse_indexed_lines(body, 2).unwrap();
        assert_eq!(out, vec!["ok".to_string()]);
    }

    #[test]
    fn normalise_target_known_aliases() {
        assert_eq!(normalise_target("EN"), "en");
        assert_eq!(normalise_target("zh-CN"), "zh");
        assert_eq!(normalise_target("中文"), "zh");
        assert_eq!(normalise_target("fr"), "");
        assert_eq!(normalise_target(""), "");
    }

    #[test]
    fn already_in_target_works() {
        assert!(already_in_target("Hello world", "en"));
        assert!(!already_in_target("你好", "en"));
        assert!(already_in_target("你好世界", "zh"));
        assert!(!already_in_target("hello", "zh"));
    }

    #[tokio::test]
    async fn translate_strings_empty_input_returns_empty() {
        let s = empty_settings();
        let out = translate_strings(&s, &[], "zh").await;
        assert!(out.is_empty());
    }

    #[tokio::test]
    async fn translate_strings_skips_when_already_in_target() {
        // Unsloth disabled in env → even if the heuristic let it
        // through, the call would short-circuit. We instead rely on
        // the heuristic to short-circuit and avoid the HTTP call.
        let s = empty_settings();
        let inputs = vec!["你好世界".to_string(), "再见".to_string()];
        let out = translate_strings(&s, &inputs, "zh").await;
        assert_eq!(out, inputs);
    }

    #[tokio::test]
    async fn translate_strings_skips_when_unknown_target() {
        let s = empty_settings();
        let inputs = vec!["Hello".to_string()];
        let out = translate_strings(&s, &inputs, "klingon").await;
        assert_eq!(out, inputs);
    }
}
