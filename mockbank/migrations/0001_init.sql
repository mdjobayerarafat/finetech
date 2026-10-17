-- mockbank schema. Lives in its own Postgres schema so the Rust backend can
-- keep using `public.*` without collisions. All money is integer fen.

CREATE SCHEMA IF NOT EXISTS mockbank;
SET search_path TO mockbank, public;

CREATE TABLE IF NOT EXISTS users (
    id                   TEXT PRIMARY KEY,
    username             TEXT NOT NULL UNIQUE,
    password_hash        TEXT NOT NULL,
    display_name         TEXT NOT NULL,
    role                 TEXT NOT NULL DEFAULT 'customer'
                          CHECK (role IN ('customer','admin')),
    safety_lock_state    TEXT NOT NULL DEFAULT 'unlocked'
                          CHECK (safety_lock_state IN ('unlocked','locked')),
    safety_lock_reason   TEXT,
    failed_mfa_count     INTEGER NOT NULL DEFAULT 0,
    risk_tolerance       TEXT NOT NULL DEFAULT 'medium',
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS sessions (
    id          TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    mfa_token   TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at  TIMESTAMPTZ NOT NULL
);
CREATE INDEX IF NOT EXISTS ix_sessions_user ON sessions(user_id);

CREATE TABLE IF NOT EXISTS accounts (
    id                       TEXT PRIMARY KEY,
    user_id                  TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    display_name             TEXT NOT NULL,
    masked_number            TEXT NOT NULL,
    currency                 TEXT NOT NULL DEFAULT 'CNY',
    available_balance_fen    BIGINT NOT NULL DEFAULT 0,
    reserved_balance_fen     BIGINT NOT NULL DEFAULT 0,
    status                   TEXT NOT NULL DEFAULT 'active'
                              CHECK (status IN ('active','frozen','closed')),
    created_at               TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS ix_accounts_user ON accounts(user_id);

CREATE TABLE IF NOT EXISTS transactions (
    id           TEXT PRIMARY KEY,
    account_id   TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    amount_fen   BIGINT NOT NULL,           -- signed: credit>0, debit<0
    direction    TEXT NOT NULL CHECK (direction IN ('credit','debit')),
    category     TEXT NOT NULL,
    merchant     TEXT NOT NULL,
    memo         TEXT NOT NULL DEFAULT '',
    occurred_at  TIMESTAMPTZ NOT NULL,
    status       TEXT NOT NULL DEFAULT 'posted'
                  CHECK (status IN ('posted','pending','reversed'))
);
CREATE INDEX IF NOT EXISTS ix_tx_account ON transactions(account_id);
CREATE INDEX IF NOT EXISTS ix_tx_occurred ON transactions(occurred_at);

CREATE TABLE IF NOT EXISTS payees (
    id               TEXT PRIMARY KEY,
    owner_user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    display_name     TEXT NOT NULL,
    phone_tail       TEXT,
    mock_destination TEXT NOT NULL,
    verified         BOOLEAN NOT NULL DEFAULT FALSE,
    first_used_at    TIMESTAMPTZ,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS ix_payees_owner ON payees(owner_user_id);

CREATE TABLE IF NOT EXISTS transfers (
    id                  TEXT PRIMARY KEY,
    user_id             TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    source_account_id   TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    payee_id            TEXT NOT NULL REFERENCES payees(id) ON DELETE CASCADE,
    amount_fen          BIGINT NOT NULL,
    memo                TEXT NOT NULL DEFAULT '',
    idempotency_key     TEXT NOT NULL UNIQUE,
    status              TEXT NOT NULL DEFAULT 'posted'
                          CHECK (status IN ('posted','failed','reversed')),
    tier                TEXT NOT NULL CHECK (tier IN ('green','yellow','red')),
    posted_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS ix_transfers_user_created ON transfers(user_id, created_at);

CREATE TABLE IF NOT EXISTS subscriptions (
    id                     TEXT PRIMARY KEY,
    user_id                TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    merchant               TEXT NOT NULL,
    estimated_amount_fen   BIGINT NOT NULL,
    interval_days          INTEGER NOT NULL,
    next_expected_at       TIMESTAMPTZ NOT NULL,
    last_charge_at         TIMESTAMPTZ,
    cancellation_status    TEXT NOT NULL DEFAULT 'active'
                            CHECK (cancellation_status IN ('active','cancellation_requested','cancelled','failed')),
    cancelled_at           TIMESTAMPTZ,
    evidence               JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at             TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS ix_subscriptions_user ON subscriptions(user_id);

CREATE TABLE IF NOT EXISTS audit_events (
    id                  TEXT PRIMARY KEY,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    actor_user_id       TEXT,
    request_id          TEXT,
    action              TEXT NOT NULL,
    tool                TEXT,
    parameters_masked   JSONB NOT NULL DEFAULT '{}'::jsonb,
    tier                TEXT,
    result              TEXT NOT NULL CHECK (result IN ('ok','error','denied','blocked')),
    error_code          TEXT,
    prev_hash           TEXT NOT NULL DEFAULT '',
    hash                TEXT NOT NULL UNIQUE
);
CREATE INDEX IF NOT EXISTS ix_audit_created ON audit_events(created_at);
CREATE INDEX IF NOT EXISTS ix_audit_actor ON audit_events(actor_user_id);

CREATE TABLE IF NOT EXISTS safety_events (
    id         TEXT PRIMARY KEY,
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind       TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    metadata   JSONB NOT NULL DEFAULT '{}'::jsonb
);
CREATE INDEX IF NOT EXISTS ix_safety_user ON safety_events(user_id);
