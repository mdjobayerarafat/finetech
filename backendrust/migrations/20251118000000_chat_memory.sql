-- BankPilot chat memory — per-user long-term recall for the agent.
--
-- Stores every chat turn (user + assistant) for the user, plus a small
-- embedding vector so we can later "remember" what we discussed and
-- inject prior context into the system prompt.
--
-- Embedding storage note: pgvector is not available on the live DB
-- (verified — `CREATE EXTENSION vector` fails with "extension control
-- file missing"). Until pgvector is installed, we store the embedding
-- as a portable `real[]` and run cosine similarity in Rust. When
-- pgvector is enabled later, the migration to add
--   embedding_v vector(N)
-- is a one-shot, and the application code branches on which column
-- is populated.
--
-- Schema notes:
--   - `embedding` is length EMBED_DIM (64). The Rust side asserts this
--     on every insert; the DB CHECK mirrors the constraint so a stray
--     manual INSERT also fails loudly.
--   - `tsvector` is auto-populated by trigger for the optional lexical
--     fallback when cosine yields no match.
--   - `summary` is a short human-readable caption the application can
--     set on write (e.g. "asked about NetStream Premium cancellation").
--   - `deleted_at` lets a user forget a turn without losing the row
--     for audit; recall queries filter on `deleted_at IS NULL`.

CREATE TABLE IF NOT EXISTS chat_memory (
    id              TEXT PRIMARY KEY,
    user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    message_id      TEXT,
    role            TEXT NOT NULL CHECK (role IN ('user','assistant','system')),
    text            TEXT NOT NULL,
    summary         TEXT,
    intent          TEXT,
    embedding       REAL[] NOT NULL,
    tsv             tsvector,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at      TIMESTAMPTZ
);

-- Length check: matches the application's EMBED_DIM constant. 64 keeps
-- the table compact and the cosine math in microseconds.
ALTER TABLE chat_memory
    DROP CONSTRAINT IF EXISTS chat_memory_embedding_len;
ALTER TABLE chat_memory
    ADD CONSTRAINT chat_memory_embedding_len
    CHECK (array_length(embedding, 1) = 64);

CREATE INDEX IF NOT EXISTS ix_chat_memory_user_created
    ON chat_memory (user_id, created_at DESC)
    WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS ix_chat_memory_user_conv
    ON chat_memory (user_id, conversation_id, created_at)
    WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS ix_chat_memory_tsv
    ON chat_memory USING gin (tsv)
    WHERE deleted_at IS NULL;

-- Auto-populate the tsvector column. `simple` is intentional — we want
-- multilingual recall (EN + zh) and a custom dictionary would need
-- per-locale config. The cosine path is the primary recall; tsvector
-- is the fallback.
CREATE OR REPLACE FUNCTION chat_memory_tsv_update() RETURNS trigger AS $$
BEGIN
    NEW.tsv :=
        setweight(to_tsvector('simple', coalesce(NEW.summary, '')), 'A') ||
        setweight(to_tsvector('simple', coalesce(NEW.text, '')), 'B');
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_chat_memory_tsv ON chat_memory;
CREATE TRIGGER trg_chat_memory_tsv
    BEFORE INSERT OR UPDATE ON chat_memory
    FOR EACH ROW EXECUTE FUNCTION chat_memory_tsv_update();
