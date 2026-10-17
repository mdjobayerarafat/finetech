//! Advisor-mode retriever: ranked lexical search over the policy corpus.
//!
//! Backend is `tsvector` + `pg_trgm` (NOT pgvector — pgvector isn't installed
//! on the live DB; the retriever interface is vector-shape compatible so a
//! future migration to embedding-based search swaps the implementation
//! without changing call sites).
//!
//! The corpus is small (a few hundred KB of policy text). For each query
//! the retriever returns the top-N chunks with a `score` in [0, 1] and the
//! raw `content`; the LLM system prompt renders them as a `RETRIEVED` block
//! and the chat route surfaces the sources in the answer.

use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RetrievedChunk {
    pub id: String,
    pub source: String,
    pub section: Option<String>,
    pub content: String,
    pub score: f32,
}

/// One search hit with citation metadata the chat can render as footnotes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    pub source: String,
    pub section: Option<String>,
    pub snippet: String,
    pub score: f32,
}

pub async fn search(
    pool: &PgPool,
    query: &str,
    limit: i64,
) -> Result<Vec<RetrievedChunk>, sqlx::Error> {
    if query.trim().is_empty() {
        return Ok(vec![]);
    }
    // The combined ranking uses ts_rank on the tsv column plus a trigram
    // similarity floor for typo tolerance.
    let rows: Vec<RetrievedChunk> = sqlx::query_as(
        "SELECT id, source, section, content, \
                (ts_rank(tsv, plainto_tsquery('simple', $1)) \
                 + similarity(content, $1) * 0.4)::real AS score \
         FROM document_chunks \
         WHERE tsv @@ plainto_tsquery('simple', $1) \
            OR similarity(content, $1) > 0.3 \
         ORDER BY score DESC LIMIT $2",
    )
    .bind(query)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Convenience wrapper that returns just the citation metadata for the
/// agent's response payload.
pub async fn citations(
    pool: &PgPool,
    query: &str,
    limit: i64,
) -> Result<Vec<Citation>, sqlx::Error> {
    let chunks = search(pool, query, limit).await?;
    Ok(chunks
        .into_iter()
        .map(|c| Citation {
            source: c.source,
            section: c.section,
            snippet: snippet(&c.content, 240),
            score: c.score,
        })
        .collect())
}

fn snippet(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let cut = s.char_indices().nth(max).map(|(i, _)| i).unwrap_or(s.len());
    format!("{}…", &s[..cut])
}

/// Ingest one chunk into the corpus. Used by the seed / admin routes.
pub async fn ingest(
    pool: &PgPool,
    source: &str,
    section: Option<&str>,
    language: &str,
    content: &str,
) -> Result<String, sqlx::Error> {
    let id = format!("dc_{}", crate::ids::new_id("dc").trim_start_matches("dc_"));
    sqlx::query(
        "INSERT INTO document_chunks (id, source, section, language, content) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(&id)
    .bind(source)
    .bind(section)
    .bind(language)
    .bind(content)
    .execute(pool)
    .await?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snippet_truncates() {
        let s = "a".repeat(500);
        let out = snippet(&s, 240);
        assert!(out.ends_with('…'));
        assert!(out.len() <= 244); // 240 chars + 3-byte ellipsis
    }

    #[test]
    fn snippet_passthrough() {
        let s = "short";
        assert_eq!(snippet(s, 240), s);
    }
}
