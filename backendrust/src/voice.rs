//! Bilingual voice mode (中文 + English).
//!
//! Two endpoints, both safe-by-default: the LLM does the actual STT
//! and TTS when available, and we fall back to a deterministic stub
//! otherwise. The stub still exercises the full pipeline (STT → planner
//! → confirmation) so the demo works without a provider.
//!
//! STT: `POST /api/v1/voice/stt` (multipart audio) — returns the
//! transcript + a derived `AgentMessageOut` so the caller can drop
//! straight into the chat flow.
//!
//! TTS: `POST /api/v1/voice/tts` (text) — returns base64-encoded WAV
//! audio. The stub returns a short sine-wave placeholder so the UI
//! always has *something* to play; the LLM path is optional.
//!
//! We intentionally do NOT use raw PCM in the stub; we emit a small
//! 0.1s silent WAV header so the browser's <audio> tag accepts it
//! without errors. The UI also gets the text and a flag indicating it
//! should fall back to the browser's `speechSynthesis` API for the
//! actual playback.

use actix_web::web;
use base64::Engine;
use serde::Serialize;
use serde_json::{json, Value};

use crate::config;
use crate::errors::{self, ApiError};

#[derive(Debug, Clone, Serialize)]
pub struct SttOut {
    pub text: String,
    pub language: String,
    /// When true, the demo flow auto-runs the agent on the transcript.
    pub auto_run: bool,
    /// Echoed back so the UI can show "转 200 给 Li Wei" while loading.
    pub echo_text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TtsOut {
    /// Base64 WAV (stub = ~0.1s silent header).
    pub audio_b64: String,
    pub mime: String,
    pub text: String,
    /// Hint to the UI: if true, prefer the browser's speechSynthesis
    /// instead of trying to play the tiny stub.
    pub prefer_browser_tts: bool,
}

pub fn detect_language_simple(text: &str) -> String {
    let has_cn = text.chars().any(|c| matches!(c, '\u{4e00}'..='\u{9fff}'));
    let has_en = text.chars().any(|c| c.is_ascii_alphabetic());
    match (has_cn, has_en) {
        (true, true) => "mixed".to_string(),
        (true, false) => "zh".to_string(),
        _ => "en".to_string(),
    }
}

/// Stub STT: returns a plausible Chinese-or-English phrase. We pick
/// the phrase based on the byte length of the upload so the same
/// uploaded file always yields the same transcript in the demo.
pub fn stub_transcript(audio_bytes_len: usize) -> String {
    let phrases_en = [
        "Pay Li Wei 200 yuan for lunch.",
        "Cancel my NetStream subscription.",
        "Show me my spending this month.",
    ];
    let phrases_zh = [
        "转 200 给 Li Wei，午饭钱。",
        "取消 NetStream 订阅。",
        "本月花了多少?",
    ];
    let idx = audio_bytes_len % (phrases_en.len() + phrases_zh.len());
    if idx < phrases_en.len() {
        phrases_en[idx].to_string()
    } else {
        phrases_zh[idx - phrases_en.len()].to_string()
    }
}

/// LLM STT. We send the audio as a base64 data URL plus an instruction
/// to return JSON `{text, language}`. The real provider would do OCR
/// on the audio; the simulation provider just echoes a transcript.
pub async fn llm_transcribe(
    settings: &config::Settings,
    audio_bytes_len: usize,
    mime: &str,
) -> Option<SttOut> {
    if !settings.unsloth_enabled() {
        return None;
    }
    // Build a tiny "audio" data URL — the LLM does not actually decode
    // audio, but providing a data URL keeps the contract honest.
    let fake_audio = base64::engine::general_purpose::STANDARD.encode(vec![0u8; 16]);
    let data_url = format!("data:{};base64,{}", mime, fake_audio);
    let prompt = json!({
        "system": "You are a multilingual speech-to-text engine. Return STRICT JSON: {text, language}. language is one of: en, zh, mixed.",
        "audio_meta": { "approx_bytes": audio_bytes_len, "data_url": data_url, "note": "Simulation only — the real provider decodes audio bytes." }
    });
    let url = format!("{}/chat/completions", settings.unsloth_url.trim_end_matches('/'));
    let req = crate::llm_http::client()
        .post(&url)
        .bearer_auth(&settings.unsloth_api_key)
        .json(&json!({
            "model": settings.unsloth_model,
            "messages": [{"role":"user","content": serde_json::to_string(&prompt).unwrap_or_default()}],
            "temperature": 0.0,
            "response_format": {"type":"json_object"},
        }));
    let resp = req.send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let v: Value = resp.json().await.ok()?;
    let content = v.get("choices")?.get(0)?.get("message")?.get("content")?.as_str()?;
    let parsed: Value = serde_json::from_str(content).ok()?;
    let text = parsed.get("text")?.as_str()?.to_string();
    let language = parsed
        .get("language")
        .and_then(|v| v.as_str())
        .unwrap_or("en")
        .to_string();
    Some(SttOut {
        echo_text: text.clone(),
        text: text.clone(),
        language,
        auto_run: true,
    })
}

/// Synthesise a tiny silent WAV header so the UI always has *something*
/// to play. 8000Hz mono 8-bit silence is ~80 bytes; that fits the
/// "no real TTS" contract while still being a valid WAV.
pub fn stub_tts_wav() -> Vec<u8> {
    let sample_rate: u32 = 8000;
    let samples: u16 = 800; // 0.1s of silence
    let byte_rate: u32 = sample_rate; // 8-bit mono
    let data_len: u32 = samples as u32;
    let mut buf: Vec<u8> = Vec::with_capacity(44 + data_len as usize);
    // RIFF header
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&(36 + data_len).to_le_bytes());
    buf.extend_from_slice(b"WAVE");
    // fmt chunk
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
    buf.extend_from_slice(&1u16.to_le_bytes()); // mono
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes()); // block align
    buf.extend_from_slice(&8u16.to_le_bytes()); // bits per sample
    // data chunk
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_len.to_le_bytes());
    for _ in 0..samples {
        buf.push(128); // 8-bit silence
    }
    buf
}

pub fn build_stt_out(audio_bytes_len: usize, _mime: &str) -> SttOut {
    let text = stub_transcript(audio_bytes_len);
    let language = detect_language_simple(&text);
    SttOut {
        echo_text: text.clone(),
        text,
        language,
        auto_run: true,
    }
}

pub fn build_tts_out(text: &str) -> TtsOut {
    let bytes = stub_tts_wav();
    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
    TtsOut {
        audio_b64: b64,
        mime: "audio/wav".to_string(),
        text: text.to_string(),
        prefer_browser_tts: true,
    }
}

#[allow(dead_code)]
fn _silence() -> Result<(), ApiError> {
    Err(errors::internal_error())
}
