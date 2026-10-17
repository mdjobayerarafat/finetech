-- BankPilot agent v2 — slots, conversations, LLM audit, retrieval, executor
-- status states, and result-dependent DAG.
--
-- Designed to be applied once. All tables are additive. Default values and
-- CHECK constraints match existing enum vocabulary so old code still works.
--
-- Note: pgvector is NOT available on the live DB (verified), so document
-- retrieval uses PostgreSQL's built-in tsvector + pg_trgm for ranked lexical
-- search. If pgvector is later installed, an `embedding vector(384)` column
-- can be added in a follow-up migration and the retriever can be switched
-- to vector similarity without changing the application code surface.

-- --- pgcrypto / trigram for lexical retrieval --------------------------------
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- --- LLM call audit --------------------------------------------------------
-- Every call to the LLM (Unsloth or stub) gets one row. Stores the prompt
-- metadata, model, latency, outcome, and a fingerprint of the prompt so we
-- can later detect cacheable / repeated calls.
CREATE TABLE IF NOT EXISTS llm_call_audit (
    id              TEXT PRIMARY KEY,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    actor_user_id   TEXT REFERENCES users(id) ON DELETE SET NULL,
    request_id      TEXT,
    conversation_id TEXT,
    provider        TEXT NOT NULL,                 -- 'unsloth' | 'stub'
    model           TEXT NOT NULL,
    language_hint   TEXT,
    prompt_fingerprint TEXT NOT NULL,              -- sha256 of canonical prompt
    prompt_tokens_estimate INTEGER,
    response_bytes  INTEGER,
    duration_ms     INTEGER NOT NULL,
    intent          TEXT,
    success         BOOLEAN NOT NULL,
    error_message   TEXT,
    used_fallback   BOOLEAN NOT NULL DEFAULT FALSE
);
CREATE INDEX IF NOT EXISTS ix_llm_call_audit_created ON llm_call_audit(created_at);
CREATE INDEX IF NOT EXISTS ix_llm_call_audit_actor_created ON llm_call_audit(actor_user_id, created_at);

-- --- Multi-turn conversations -----------------------------------------------
-- Each chat session is a conversation. Messages are appended chronologically;
-- the planner pulls recent context when resolving slots.
CREATE TABLE IF NOT EXISTS conversations (
    id              TEXT PRIMARY KEY,
    user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title           TEXT,
    last_message_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    slot_state      TEXT NOT NULL DEFAULT '{}',    -- JSON: { tool: { slot_name: value } }
    pending_workflow_id TEXT,                      -- TEXT not FK; created lazily
    advisor_mode    BOOLEAN NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS ix_conversations_user_lastmsg ON conversations(user_id, last_message_at DESC);

CREATE TABLE IF NOT EXISTS conversation_messages (
    id              TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role            TEXT NOT NULL CHECK (role IN ('user','assistant','system','tool')),
    content         TEXT NOT NULL,
    proposal_json   TEXT,                          -- assistant proposal payload, if any
    workflow_id     TEXT,
    tool_calls_json TEXT,                          -- tool calls made by the assistant
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS ix_conversation_messages_conv_created ON conversation_messages(conversation_id, created_at);

-- --- Slot definitions + per-message slot bindings ---------------------------
-- Tool authors can declare their required slots in code; this table mirrors
-- the most recently filled values per conversation so we can ask targeted
-- follow-up questions ("What amount?") instead of a generic "more info".
CREATE TABLE IF NOT EXISTS slot_bindings (
    id              TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    tool            TEXT NOT NULL,
    slot_name       TEXT NOT NULL,
    slot_value      TEXT NOT NULL,
    source          TEXT NOT NULL DEFAULT 'user',  -- 'user' | 'inferred' | 'context'
    confidence      REAL NOT NULL DEFAULT 1.0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (conversation_id, tool, slot_name)
);
CREATE INDEX IF NOT EXISTS ix_slot_bindings_conv ON slot_bindings(conversation_id);

CREATE TABLE IF NOT EXISTS slot_questions (
    id              TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    tool            TEXT NOT NULL,
    slot_name       TEXT NOT NULL,
    question        TEXT NOT NULL,
    asked_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    answered_at     TIMESTAMPTZ,
    UNIQUE (conversation_id, tool, slot_name)
);

-- --- Workflow result-dependent DAG -----------------------------------------
-- We already have workflow_nodes.dependencies (a JSON array of node IDs).
-- This table records the OUTCOME of each prior node that downstream nodes
-- declared a dependency on, so the executor can short-circuit (skip) nodes
-- whose dependency produced a failure.
CREATE TABLE IF NOT EXISTS workflow_node_outcomes (
    id              TEXT PRIMARY KEY,
    workflow_id     TEXT NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    node_id         TEXT NOT NULL REFERENCES workflow_nodes(id) ON DELETE CASCADE,
    consumer_node_id TEXT NOT NULL REFERENCES workflow_nodes(id) ON DELETE CASCADE,
    required_status TEXT NOT NULL,                 -- 'completed' | 'succeeded' | 'any'
    observed_status TEXT NOT NULL,
    blocked         BOOLEAN NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (consumer_node_id, node_id)
);
CREATE INDEX IF NOT EXISTS ix_workflow_node_outcomes_wf ON workflow_node_outcomes(workflow_id);

-- --- Background workflow executor ------------------------------------------
-- The route handlers currently advance workflows synchronously inside
-- `agent_message`. To make workflows survive across requests, we add a
-- queue-like table the executor polls. `next_attempt_at` supports backoff.
CREATE TABLE IF NOT EXISTS workflow_jobs (
    id              TEXT PRIMARY KEY,
    workflow_id     TEXT NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    node_id         TEXT NOT NULL REFERENCES workflow_nodes(id) ON DELETE CASCADE,
    status          TEXT NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending','running','succeeded','failed','skipped','compensated')),
    attempts        INTEGER NOT NULL DEFAULT 0,
    last_error      TEXT,
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    locked_by       TEXT,
    locked_until    TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS ix_workflow_jobs_next ON workflow_jobs(status, next_attempt_at);
CREATE INDEX IF NOT EXISTS ix_workflow_jobs_workflow ON workflow_jobs(workflow_id);

-- --- Advisor-mode retrieval corpus -----------------------------------------
-- The advisor mode asks natural-language questions about policies. We store
-- small text chunks (≤ 4000 chars) and retrieve them by tsvector ranking.
-- When pgvector is later added, an `embedding vector(N)` column can be
-- added without changing the application code shape.
CREATE TABLE IF NOT EXISTS document_chunks (
    id              TEXT PRIMARY KEY,
    source          TEXT NOT NULL,                 -- 'policy.md' | 'help.md' etc
    section         TEXT,
    language        TEXT NOT NULL DEFAULT 'en',
    content         TEXT NOT NULL,
    tsv             tsvector,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS ix_document_chunks_tsv ON document_chunks USING gin(tsv);
CREATE INDEX IF NOT EXISTS ix_document_chunks_source ON document_chunks(source);

-- Convenience trigger to keep tsv fresh on insert/update.
CREATE OR REPLACE FUNCTION document_chunks_tsv_update() RETURNS trigger AS $$
BEGIN
    NEW.tsv :=
        setweight(to_tsvector('simple', coalesce(NEW.source, '')), 'A') ||
        setweight(to_tsvector('simple', coalesce(NEW.section, '')), 'B') ||
        setweight(to_tsvector('simple', coalesce(NEW.content, '')), 'C');
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_document_chunks_tsv ON document_chunks;
CREATE TRIGGER trg_document_chunks_tsv
    BEFORE INSERT OR UPDATE ON document_chunks
    FOR EACH ROW EXECUTE FUNCTION document_chunks_tsv_update();

-- --- Extend workflow_nodes with executor-friendly columns ------------------
-- These are nullable so the existing routes (which never set them) keep
-- working unchanged.
ALTER TABLE workflow_nodes
    ADD COLUMN IF NOT EXISTS depends_on          TEXT NOT NULL DEFAULT '[]',  -- JSON array of node ids
    ADD COLUMN IF NOT EXISTS requires            TEXT NOT NULL DEFAULT '[]',  -- JSON array of {dep_node_id, required_status}
    ADD COLUMN IF NOT EXISTS on_failure         TEXT NOT NULL DEFAULT 'fail', -- 'fail' | 'skip' | 'compensate'
    ADD COLUMN IF NOT EXISTS started_at          TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS finished_at         TIMESTAMPTZ;
