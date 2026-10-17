//! Multi-turn conversations, slot bindings, and question tracking.
//!
//! Each chat session maps to one `conversations` row. Messages are appended
//! in order; the planner reads the recent N messages to disambiguate slot
//! fills across turns. Slot bindings are the conversation-scoped store of
//! "the user has already told us X for tool Y", so a follow-up
//! "to Li Wei" naturally fills the `payee_id` slot for the next planner call.
//!
//! The `conversations` table also stores `slot_state` (a JSON dump of the
//! current binding snapshot) so the route can render a conversation list
//! without re-reading the `slot_bindings` table on every call.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::ids;
use crate::models::User;
use crate::tool_spec::{self, ToolSpec};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Conversation {
    pub id: String,
    pub user_id: String,
    pub title: Option<String>,
    pub last_message_at: DateTime<Utc>,
    pub slot_state: String,
    pub pending_workflow_id: Option<String>,
    pub advisor_mode: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ConversationMessage {
    pub id: String,
    pub conversation_id: String,
    pub user_id: String,
    pub role: String,
    pub content: String,
    pub proposal_json: Option<String>,
    pub workflow_id: Option<String>,
    pub tool_calls_json: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SlotBinding {
    pub id: String,
    pub conversation_id: String,
    pub tool: String,
    pub slot_name: String,
    pub slot_value: String,
    pub source: String,
    pub confidence: f32,
    pub created_at: DateTime<Utc>,
}

/// Find an existing conversation for the user, or create a new one. When
/// `conversation_id` is provided, look it up directly; otherwise return the
/// most recent conversation, creating one if none exists.
pub async fn get_or_create(
    pool: &PgPool,
    user: &User,
    conversation_id: Option<&str>,
) -> Result<Conversation, sqlx::Error> {
    if let Some(cid) = conversation_id {
        if let Some(c) = sqlx::query_as::<_, Conversation>(
            "SELECT * FROM conversations WHERE id = $1 AND user_id = $2",
        )
        .bind(cid)
        .bind(&user.id)
        .fetch_optional(pool)
        .await?
        {
            return Ok(c);
        }
    }
    if let Some(c) = sqlx::query_as::<_, Conversation>(
        "SELECT * FROM conversations WHERE user_id = $1 ORDER BY last_message_at DESC LIMIT 1",
    )
    .bind(&user.id)
    .fetch_optional(pool)
    .await?
    {
        return Ok(c);
    }
    let id = ids::new_id("conv");
    let now = Utc::now();
    sqlx::query(
        "INSERT INTO conversations (id, user_id, title, last_message_at, slot_state, advisor_mode, created_at) \
         VALUES ($1, $2, $3, $4, '{}', FALSE, $5)",
    )
    .bind(&id)
    .bind(&user.id)
    .bind::<Option<String>>(None)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(Conversation {
        id,
        user_id: user.id.clone(),
        title: None,
        last_message_at: now,
        slot_state: "{}".to_string(),
        pending_workflow_id: None,
        advisor_mode: false,
        created_at: now,
    })
}

/// Append a chat message and bump the conversation's `last_message_at`.
pub async fn append_message(
    pool: &PgPool,
    conv: &Conversation,
    user: &User,
    role: &str,
    content: &str,
    proposal: Option<&Value>,
    workflow_id: Option<&str>,
    tool_calls: Option<&Value>,
) -> Result<ConversationMessage, sqlx::Error> {
    let id = ids::new_id("msg");
    let now = Utc::now();
    let proposal_str = proposal.map(|v| serde_json::to_string(v).unwrap_or_else(|_| "{}".into()));
    let tool_calls_str = tool_calls.map(|v| serde_json::to_string(v).unwrap_or_else(|_| "[]".into()));
    let proposal_db = proposal_str.clone();
    let tool_calls_db = tool_calls_str.clone();
    sqlx::query(
        "INSERT INTO conversation_messages (id, conversation_id, user_id, role, content, proposal_json, workflow_id, tool_calls_json, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(&id)
    .bind(&conv.id)
    .bind(&user.id)
    .bind(role)
    .bind(content)
    .bind(proposal_db)
    .bind(workflow_id)
    .bind(tool_calls_db)
    .bind(now)
    .execute(pool)
    .await?;
    sqlx::query("UPDATE conversations SET last_message_at = $1 WHERE id = $2")
        .bind(now)
        .bind(&conv.id)
        .execute(pool)
        .await?;
    Ok(ConversationMessage {
        id,
        conversation_id: conv.id.clone(),
        user_id: user.id.clone(),
        role: role.to_string(),
        content: content.to_string(),
        proposal_json: proposal_str,
        workflow_id: workflow_id.map(|s| s.to_string()),
        tool_calls_json: tool_calls_str,
        created_at: now,
    })
}

/// Load the last `n` messages (default 12) for context. The agent uses this
/// when the current user message is short or refers to "the one above".
pub async fn recent_messages(
    pool: &PgPool,
    conv: &Conversation,
    limit: i64,
) -> Result<Vec<ConversationMessage>, sqlx::Error> {
    let rows: Vec<ConversationMessage> = sqlx::query_as(
        "SELECT * FROM conversation_messages WHERE conversation_id = $1 ORDER BY created_at DESC LIMIT $2",
    )
    .bind(&conv.id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().rev().collect())
}

/// Bind a slot value to a tool/conversation pair, idempotently overwriting
/// any prior value. Also writes a corresponding `slot_questions` row marked
/// as answered. `source` is "user", "inferred", or "context".
pub async fn bind_slot(
    pool: &PgPool,
    conv: &Conversation,
    tool: &str,
    slot_name: &str,
    value: &str,
    source: &str,
    confidence: f32,
) -> Result<(), sqlx::Error> {
    let id = ids::new_id("sb");
    sqlx::query(
        "INSERT INTO slot_bindings (id, conversation_id, tool, slot_name, slot_value, source, confidence) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) \
         ON CONFLICT (conversation_id, tool, slot_name) DO UPDATE SET slot_value = EXCLUDED.slot_value, source = EXCLUDED.source, confidence = EXCLUDED.confidence",
    )
    .bind(&id)
    .bind(&conv.id)
    .bind(tool)
    .bind(slot_name)
    .bind(value)
    .bind(source)
    .bind(confidence)
    .execute(pool)
    .await?;
    sqlx::query(
        "UPDATE slot_questions SET answered_at = NOW() WHERE conversation_id = $1 AND tool = $2 AND slot_name = $3",
    )
    .bind(&conv.id)
    .bind(tool)
    .bind(slot_name)
    .execute(pool)
    .await?;
    refresh_slot_state(pool, conv).await?;
    Ok(())
}

/// Persist a question that the planner is asking the user. Idempotent on
/// (conversation_id, tool, slot_name).
pub async fn record_question(
    pool: &PgPool,
    conv: &Conversation,
    tool: &str,
    slot_name: &str,
    question: &str,
) -> Result<(), sqlx::Error> {
    let id = ids::new_id("sq");
    sqlx::query(
        "INSERT INTO slot_questions (id, conversation_id, tool, slot_name, question) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (conversation_id, tool, slot_name) DO UPDATE SET question = EXCLUDED.question, asked_at = NOW(), answered_at = NULL",
    )
    .bind(&id)
    .bind(&conv.id)
    .bind(tool)
    .bind(slot_name)
    .bind(question)
    .execute(pool)
    .await?;
    Ok(())
}

/// Load the latest binding for every (tool, slot_name) the user has
/// answered in this conversation. The planner consults this to backfill
/// missing slots before asking the user.
pub async fn load_bindings(
    pool: &PgPool,
    conv: &Conversation,
) -> Result<Vec<SlotBinding>, sqlx::Error> {
    let rows: Vec<SlotBinding> = sqlx::query_as(
        "SELECT * FROM slot_bindings WHERE conversation_id = $1 ORDER BY created_at ASC",
    )
    .bind(&conv.id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Decide which slots still need a value for a given tool. Consults:
/// 1. The proposed action's `arguments`.
/// 2. The conversation's prior `slot_bindings`.
/// 3. The spec's `required_slots` (vs `optional_slots`).
///
/// Returns a list of questions the planner must ask, in the same order as
/// the spec's required slots. An empty vec means the planner can build a
/// node.
pub async fn missing_slots(
    pool: &PgPool,
    conv: &Conversation,
    spec: &ToolSpec,
    proposed_args: &Value,
) -> Result<Vec<crate::tool_spec::MissingSlot>, sqlx::Error> {
    let bindings = load_bindings(pool, conv).await?;
    let mut augmented = proposed_args.clone();
    if let Value::Object(ref mut m) = augmented {
        for b in &bindings {
            if b.tool == spec.canonical_name {
                if !m.contains_key(&b.slot_name) || m.get(&b.slot_name).and_then(|v| v.as_str()).map(|s| s.is_empty()).unwrap_or(false) {
                    m.insert(b.slot_name.clone(), Value::String(b.slot_value.clone()));
                }
            }
        }
    }
    match tool_spec::validate_arguments(spec, &augmented) {
        Ok(()) => Ok(vec![]),
        Err(missing) => Ok(missing),
    }
}

async fn refresh_slot_state(pool: &PgPool, conv: &Conversation) -> Result<(), sqlx::Error> {
    let bindings = load_bindings(pool, conv).await?;
    let mut grouped: std::collections::BTreeMap<String, serde_json::Map<String, Value>> =
        std::collections::BTreeMap::new();
    for b in &bindings {
        let entry = grouped.entry(b.tool.clone()).or_default();
        entry.insert(b.slot_name.clone(), Value::String(b.slot_value.clone()));
    }
    let snapshot: serde_json::Map<String, Value> = grouped
        .into_iter()
        .map(|(tool, slots)| (tool, Value::Object(slots)))
        .collect();
    let s = serde_json::to_string(&snapshot).unwrap_or_else(|_| "{}".into());
    sqlx::query("UPDATE conversations SET slot_state = $1 WHERE id = $2")
        .bind(s)
        .bind(&conv.id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Generate a new conversation id (exposed for tests and seeds).
pub fn new_id() -> String {
    format!("conv_{}", Uuid::new_v4().simple())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn new_id_has_prefix() {
        assert!(new_id().starts_with("conv_"));
    }

    #[test]
    fn slot_bindings_serialize_roundtrip() {
        let s = json!({
            "transfer.create_pending": { "payee_id": "Li Wei", "amount_fen": "20000" }
        });
        let s_str = serde_json::to_string(&s).unwrap();
        let back: Value = serde_json::from_str(&s_str).unwrap();
        assert_eq!(s, back);
    }
}
