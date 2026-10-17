//! Per-user chat memory for the agent.
//!
//! Every chat turn is stored in the `chat_memory` table along with a
//! small embedding vector. Before each new turn the agent looks up the
//! top-K most similar prior turns and injects a short "Recent chats
//! you may be referring to" block into the system prompt.
//!
//! ## Why not pgvector?
//!
//! The live PostgreSQL doesn't have the pgvector extension installed
//! (`CREATE EXTENSION vector` fails with "extension control file
//! missing"). We therefore store embeddings as a `real[]` and run
//! cosine similarity in Rust. The schema is pgvector-shaped so a
//! future migration to `vector(N)` is a one-line change and the
//! application code switches to `<=>` operator without touching call
//! sites.
//!
//! ## Embedding quality
//!
//! The current embedding is a *hashed bag of trigrams* (56 dims) plus
//! an 8-dim one-hot for known intent families. This is intentionally
//! fast and dependency-free; it captures lexical similarity well
//! ("cancel NetStream" ≈ "NetStream cancellation") but not paraphrase.
//! The trade-off is the right one for a demo where the user benefits
//! from "I remember you asked this yesterday" even when the wording
//! shifts slightly.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::ids;

/// Must match the `chat_memory_embedding_len` CHECK constraint in the
/// migration. Changing this requires a migration that drops and
/// re-adds the CHECK after a column rewrite.
pub const EMBED_DIM: usize = 64;

/// Family of known intents that get a one-hot channel. New intents
/// fall into the 9th "other" slot.
const INTENT_FAMILIES: &[&str] = &[
    "transfer",
    "report_spending",
    "cancel_subscription",
    "list_subscriptions",
    "list_cards",
    "wealth",
    "reminder",
    "card",
];

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MemoryRow {
    pub id: String,
    pub user_id: String,
    pub conversation_id: String,
    pub message_id: Option<String>,
    pub role: String,
    pub text: String,
    pub summary: Option<String>,
    pub intent: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Embedding + score from a recall query. Returned to the caller for
/// either rendering or further LLM-side filtering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryHit {
    pub row: MemoryRow,
    pub score: f32,
}

/// Build a deterministic 64-dim embedding for a chat turn.
///
/// Layout:
///   - dims `[0..56)`  → 56 trigram buckets, each is a sign-magnitude
///     hash of (trigram, position-bucket) so the same word in two
///     positions doesn't cancel out. This is the "lexical" channel.
///   - dims `[56..64)` → 8-dim one-hot for the intent family. The 9th
///     slot stays zero (reserved for "other").
pub fn embed(text: &str, intent: Option<&str>) -> [f32; EMBED_DIM] {
    let mut v = [0f32; EMBED_DIM];
    let normalised = normalise(text);
    if normalised.is_empty() {
        return v;
    }
    // 56 trigram channels.
    let chars: Vec<char> = normalised.chars().collect();
    let mut i = 0;
    while i + 3 <= chars.len() {
        let tri: String = chars[i..i + 3].iter().collect();
        let pos_bucket = (i / 4) & 0x07; // 0..8 — extra position signal
        let h = hash32(tri.as_bytes());
        let ch = (h % 56) as usize;
        let sign = if (h >> 31) & 1 == 1 { 1.0 } else { -1.0 };
        v[ch] += sign * 1.0;
        // position-bucket contribution blended into a nearby channel
        let ch2 = ((h >> 8) % 56) as usize;
        v[ch2] += sign * 0.5 * (1.0 + pos_bucket as f32 / 8.0);
        i += 1;
    }
    // 8-dim intent one-hot.
    if let Some(label) = intent {
        let family_idx = INTENT_FAMILIES.iter().position(|f| label.starts_with(f));
        if let Some(idx) = family_idx {
            v[56 + idx] = 1.0;
        }
        // 9th family ("other") stays at index 63 = 0 implicitly.
    }
    // L2-normalise so cosine == dot product.
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
    v
}

/// Lowercase, drop punctuation/emoji, collapse whitespace.
fn normalise(s: &str) -> String {
    let lowered = s.to_lowercase();
    let mut out = String::with_capacity(lowered.len());
    let mut prev_space = false;
    for c in lowered.chars() {
        if c.is_alphanumeric() {
            out.push(c);
            prev_space = false;
        } else if c.is_whitespace() {
            if !prev_space && !out.is_empty() {
                out.push(' ');
                prev_space = true;
            }
        }
        // drop punctuation/emoji
    }
    out.trim().to_string()
}

/// Tiny FNV-1a 32-bit hash. Stable, no crypto needed.
fn hash32(bytes: &[u8]) -> u32 {
    let mut h: u32 = 0x811c9dc5;
    for b in bytes {
        h ^= *b as u32;
        h = h.wrapping_mul(0x01000193);
    }
    h
}

/// Convert a [f32; 64] to the `real[]` literal Postgres expects.
pub fn embedding_to_pg(v: &[f32; EMBED_DIM]) -> Vec<f32> {
    v.to_vec()
}

/// Cosine similarity between two pre-normalised embeddings. They come
/// pre-normalised from `embed`, so cosine ≡ dot product.
pub fn cosine(a: &[f32; EMBED_DIM], b: &[f32; EMBED_DIM]) -> f32 {
    let mut s = 0f32;
    for i in 0..EMBED_DIM {
        s += a[i] * b[i];
    }
    s
}

#[derive(Debug, Clone)]
pub struct RecordParams<'a> {
    pub user_id: &'a str,
    pub conversation_id: &'a str,
    pub message_id: Option<&'a str>,
    pub role: &'a str,        // 'user' | 'assistant' | 'system'
    pub text: &'a str,
    pub summary: Option<&'a str>,
    pub intent: Option<&'a str>,
}

/// Persist one chat turn to `chat_memory`. The embedding is computed
/// in-line; callers don't have to know EMBED_DIM.
pub async fn record_turn(pool: &PgPool, p: RecordParams<'_>) -> Result<String, sqlx::Error> {
    let id = ids::new_id("mem");
    let v = embed(p.text, p.intent);
    let vec = embedding_to_pg(&v);
    sqlx::query(
        "INSERT INTO chat_memory (id, user_id, conversation_id, message_id, role, text, summary, intent, embedding) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(&id)
    .bind(p.user_id)
    .bind(p.conversation_id)
    .bind(p.message_id)
    .bind(p.role)
    .bind(p.text)
    .bind(p.summary)
    .bind(p.intent)
    .bind(&vec)
    .execute(pool)
    .await?;
    Ok(id)
}

/// Return the top-K memory rows for the user ranked by cosine to the
/// query embedding. Excludes the same conversation (so a follow-up
/// doesn't just echo its own context), and `deleted_at IS NULL` rows.
pub async fn recall_similar(
    pool: &PgPool,
    user_id: &str,
    query: &str,
    exclude_conversation_id: Option<&str>,
    limit: i64,
) -> Result<Vec<MemoryHit>, sqlx::Error> {
    let q_emb = embed(query, None);
    let q_vec = embedding_to_pg(&q_emb);
    // We can't push cosine math into SQL without a pgvector-style
    // operator, so we pull the recent N rows and rank in Rust. N is
    // bounded so the query stays cheap on the demo DB.
    const WINDOW: i64 = 200;
    let rows: Vec<MemoryRow> = sqlx::query_as(
        "SELECT id, user_id, conversation_id, message_id, role, text, summary, intent, created_at, embedding \
         FROM chat_memory \
         WHERE user_id = $1 AND deleted_at IS NULL \
           AND ($2::text IS NULL OR conversation_id <> $2) \
         ORDER BY created_at DESC LIMIT $3",
    )
    .bind(user_id)
    .bind(exclude_conversation_id)
    .bind(WINDOW)
    .fetch_all(pool)
    .await?;
    // Read the embeddings as Vec<f32> via a follow-up pass. Cheaper than
    // the struct above; we only need the float arrays here.
    let mut scored: Vec<MemoryHit> = Vec::with_capacity(rows.len());
    for r in rows {
        let v: Vec<f32> = sqlx::query_scalar("SELECT embedding FROM chat_memory WHERE id = $1")
            .bind(&r.id)
            .fetch_one(pool)
            .await?;
        if v.len() != EMBED_DIM {
            continue;
        }
        let mut arr = [0f32; EMBED_DIM];
        arr.copy_from_slice(&v);
        let score = cosine(&q_emb, &arr);
        scored.push(MemoryHit { row: r, score });
    }
    scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit.max(0) as usize);
    Ok(scored)
}

/// Build the system-prompt prefix the agent prepends before its own
/// instructions. Returns `None` when there is nothing to recall.
pub fn render_recall_block(hits: &[MemoryHit]) -> Option<String> {
    let meaningful: Vec<&MemoryHit> = hits
        .iter()
        .filter(|h| h.score >= 0.15) // ignore near-zero noise
        .take(5)
        .collect();
    if meaningful.is_empty() {
        return None;
    }
    let mut out = String::from("## Recent chats you may be referring to\n");
    for (i, h) in meaningful.iter().enumerate() {
        let who = if h.row.role == "user" { "user" } else { "assistant" };
        let when = h.row.created_at.format("%Y-%m-%d %H:%M UTC");
        let summary = h
            .row
            .summary
            .clone()
            .unwrap_or_else(|| short_summary(&h.row.text));
        out.push_str(&format!(
            "{}. [{}] [score={:.2}] {}: {}\n",
            i + 1,
            when,
            h.score,
            who,
            summary
        ));
    }
    out.push_str(
        "\nIf the user's current message looks like a follow-up, use the matches above. Do NOT quote them verbatim — paraphrase.\n",
    );
    Some(out)
}

fn short_summary(text: &str) -> String {
    let trimmed: String = text.chars().take(120).collect();
    if trimmed.len() < text.chars().count() {
        format!("{trimmed}…")
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedding_is_l2_normalised() {
        let v = embed("cancel my NetStream Premium subscription", Some("cancel_subscription"));
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-4 || norm == 0.0, "norm={norm}");
    }

    #[test]
    fn embedding_is_deterministic() {
        let a = embed("hello world", Some("list_subscriptions"));
        let b = embed("hello world", Some("list_subscriptions"));
        assert_eq!(a, b);
    }

    #[test]
    fn similar_texts_have_higher_cosine() {
        let a = embed("cancel NetStream subscription", Some("cancel_subscription"));
        let b = embed("NetStream cancellation please", Some("cancel_subscription"));
        let c = embed("how do I freeze a card", Some("card"));
        let ab = cosine(&a, &b);
        let ac = cosine(&a, &c);
        assert!(ab > ac, "expected ab > ac; got {ab} vs {ac}");
        assert!(ab > 0.1, "ab={ab} should not be ~0");
    }

    #[test]
    fn intent_one_hot_present() {
        let v = embed("anything", Some("cancel_subscription"));
        // "cancel" is family index 2 in INTENT_FAMILIES.
        assert!(v[56 + 2] > 0.0, "intent channel empty: {:?}", &v[56..64]);
    }

    #[test]
    fn normalise_drops_punctuation() {
        assert_eq!(normalise("Hello, world! 🎉"), "hello world");
        assert_eq!(normalise("  spaces  "), "spaces");
    }
}
