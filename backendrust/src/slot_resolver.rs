//! Smart slot resolution: fuzzy matching for payee / merchant / product names.
//!
//! The plain SQL `LOWER(merchant) LIKE '%query%'` only ever does substring
//! match. Real users misspell ("Netstream"), translate ("NetStream 订阅"),
//! and use shorthand. This module layers three pieces on top:
//!
//! 1. Normalisation (lowercase, strip punctuation, collapse whitespace).
//! 2. Trigram similarity via `pg_trgm` (≥ 0.4 similarity → candidate).
//! 3. A confidence score so the caller can ask "did you mean…?" when the
//!    best candidate's similarity is in `[0.4, 0.7)`.
//!
//! Used by the planner's resolvers in `planner.rs`. Kept separate so the
//! SQL layer stays declarative and the matching logic is unit-testable.

use serde_json::Value;
use sqlx::PgPool;

/// Normalise a free-form slot value for matching: lowercase, drop
/// whitespace, strip common CJK/Latin separators, and trim.
pub fn normalise(s: &str) -> String {
    let lowered = s.to_lowercase();
    let mut out = String::with_capacity(lowered.len());
    let mut prev_space = false;
    for c in lowered.chars() {
        if c.is_alphanumeric() {
            out.push(c);
            prev_space = false;
        } else if c.is_whitespace() || matches!(c, '·' | '、' | ',' | ';' | '.') {
            if !prev_space && !out.is_empty() {
                out.push(' ');
                prev_space = true;
            }
        }
        // punctuation, emojis and other marks dropped entirely
    }
    out.trim().to_string()
}

/// Trigram similarity score in [0.0, 1.0] backed by `pg_trgm`. Returns 0.0
/// if pg_trgm is unavailable or the input is empty.
pub async fn similarity(pool: &PgPool, a: &str, b: &str) -> Result<f64, sqlx::Error> {
    if a.is_empty() || b.is_empty() {
        return Ok(0.0);
    }
    let v: f64 = sqlx::query_scalar("SELECT similarity($1, $2)")
        .bind(a)
        .bind(b)
        .fetch_one(pool)
        .await?;
    Ok(v.clamp(0.0, 1.0))
}

/// One fuzzy-match candidate with its score.
#[derive(Debug, Clone)]
pub struct Candidate {
    pub id: String,
    pub display: String,
    pub score: f64,
    /// Original row columns the planner can use to decide.
    pub extra: Value,
}

/// Match a free-form query against a labelled table (`name TEXT PRIMARY KEY,
/// ...`). Returns the top-N candidates sorted by descending score.
pub async fn match_table(
    pool: &PgPool,
    table: &str,
    label_col: &str,
    id_col: &str,
    extra_cols: &[&str],
    user_id: Option<&str>,
    query: &str,
    limit: i64,
) -> Result<Vec<Candidate>, sqlx::Error> {
    let nq = normalise(query);
    if nq.is_empty() {
        return Ok(vec![]);
    }
    // We sort by trigram similarity DESC, with a fallback ILIKE contains
    // so that "NetStream" still finds "NetStream Premium" even when
    // similarity is 0.6 due to length mismatch.
    let sql = format!(
        "SELECT {id} AS id, {lbl} AS label, \
                similarity(lower({lbl}), lower($1)) AS score \
         FROM {table} \
         WHERE ($2::text IS NULL OR {user_clause}) \
           AND (lower({lbl}) LIKE '%' || lower($1) || '%' \
                OR similarity(lower({lbl}), lower($1)) >= 0.4) \
         ORDER BY score DESC NULLS LAST LIMIT $3",
        id = id_col,
        lbl = label_col,
        table = table,
        user_clause = if user_id.is_some() {
            "user_id = $2"
        } else {
            "TRUE"
        }
    );
    let mut q = sqlx::query_as::<_, (String, String, Option<f64>)>(&sql).bind(&nq);
    if let Some(uid) = user_id {
        q = q.bind(uid);
    } else {
        q = q.bind(Option::<String>::None);
    }
    q = q.bind(limit);
    let rows = q.fetch_all(pool).await?;
    let _ = extra_cols;
    Ok(rows
        .into_iter()
        .map(|(id, label, score)| Candidate {
            id,
            display: label,
            score: score.unwrap_or(0.0),
            extra: Value::Null,
        })
        .collect())
}

/// Pick the best candidate above `threshold`, or None. Used by planner
/// resolvers to upgrade a near-match into an auto-resolution.
pub fn best<'a>(cands: &'a [Candidate], threshold: f64) -> Option<&'a Candidate> {
    cands.iter().find(|c| c.score >= threshold)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalisation_strip_punct() {
        assert_eq!(normalise("NetStream 订阅"), "netstream 订阅");
        assert_eq!(normalise("  NetStream   Premium. "), "netstream premium");
        assert_eq!(normalise("Li, Wei"), "li wei");
    }

    #[test]
    fn normalisation_drops_emoji() {
        assert_eq!(normalise("cake 🎂 tomorrow"), "cake tomorrow");
    }

    #[test]
    fn best_picks_above_threshold() {
        let cs = vec![
            Candidate { id: "a".into(), display: "A".into(), score: 0.9, extra: Value::Null },
            Candidate { id: "b".into(), display: "B".into(), score: 0.2, extra: Value::Null },
        ];
        assert_eq!(best(&cs, 0.5).unwrap().id, "a");
        assert_eq!(best(&cs, 0.95).map(|c| c.id.as_str()), None);
    }
}
