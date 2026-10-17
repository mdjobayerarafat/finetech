-- BankPilot initial schema (PostgreSQL). Mirrors the SQLAlchemy models in backend/app/models.py.
-- Money columns are BigInteger *_fen; never f64 for ledger math.

CREATE TABLE IF NOT EXISTS users (
    id                   TEXT PRIMARY KEY,
    username             TEXT NOT NULL UNIQUE,
    password_hash        TEXT NOT NULL,
    display_name         TEXT NOT NULL,
    preferred_language   TEXT NOT NULL DEFAULT 'en',
    role                 TEXT NOT NULL DEFAULT 'customer',
    safety_lock_state    TEXT NOT NULL DEFAULT 'unlocked',
    safety_lock_reason   TEXT,
    failed_mfa_count     INTEGER NOT NULL DEFAULT 0,
    risk_tolerance       TEXT NOT NULL DEFAULT 'medium',
    horizon              TEXT NOT NULL DEFAULT 'medium',
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS sessions (
    id            TEXT PRIMARY KEY,
    user_id       TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    csrf_token    TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at    TIMESTAMPTZ NOT NULL
);

CREATE TABLE IF NOT EXISTS accounts (
    id                       TEXT PRIMARY KEY,
    user_id                  TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    display_name             TEXT NOT NULL,
    masked_number            TEXT NOT NULL,
    currency                 TEXT NOT NULL DEFAULT 'CNY',
    available_balance_fen    BIGINT NOT NULL DEFAULT 0,
    reserved_balance_fen     BIGINT NOT NULL DEFAULT 0,
    created_at               TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS ix_accounts_user ON accounts(user_id);

CREATE TABLE IF NOT EXISTS transactions (
    id           TEXT PRIMARY KEY,
    account_id   TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    amount_fen   BIGINT NOT NULL,
    direction    TEXT NOT NULL CHECK (direction IN ('credit','debit')),
    category     TEXT NOT NULL,
    merchant     TEXT NOT NULL,
    memo         TEXT NOT NULL DEFAULT '',
    occurred_at  TIMESTAMPTZ NOT NULL,
    status       TEXT NOT NULL DEFAULT 'posted' CHECK (status IN ('posted','pending','reversed'))
);
CREATE INDEX IF NOT EXISTS ix_transactions_account ON transactions(account_id);
CREATE INDEX IF NOT EXISTS ix_transactions_occurred ON transactions(occurred_at);
CREATE INDEX IF NOT EXISTS ix_transactions_account_occurred ON transactions(account_id, occurred_at);

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
    source_account_id   TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    payee_id            TEXT NOT NULL REFERENCES payees(id) ON DELETE CASCADE,
    amount_fen          BIGINT NOT NULL,
    memo                TEXT NOT NULL DEFAULT '',
    schedule            TIMESTAMPTZ,
    idempotency_key     TEXT NOT NULL UNIQUE,
    status              TEXT NOT NULL DEFAULT 'pending_approval'
                        CHECK (status IN ('pending_approval','approved','scheduled','posted','failed','reversed')),
    posted_at           TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS subscriptions (
    id                     TEXT PRIMARY KEY,
    user_id                TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    merchant               TEXT NOT NULL,
    estimated_amount_fen   BIGINT NOT NULL,
    interval_days          INTEGER NOT NULL,
    next_expected_at       TIMESTAMPTZ NOT NULL,
    last_charge_at         TIMESTAMPTZ,
    evidence               JSONB NOT NULL DEFAULT '{}'::jsonb,
    cancellation_status    TEXT NOT NULL DEFAULT 'active'
                            CHECK (cancellation_status IN ('active','cancellation_requested','cancelled','failed')),
    cancelled_at           TIMESTAMPTZ,
    created_at             TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS ix_subscriptions_user ON subscriptions(user_id);

CREATE TABLE IF NOT EXISTS reminders (
    id                     TEXT PRIMARY KEY,
    user_id                TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind                   TEXT NOT NULL,
    subject                TEXT NOT NULL,
    due_at                 TIMESTAMPTZ NOT NULL,
    status                 TEXT NOT NULL DEFAULT 'scheduled'
                            CHECK (status IN ('scheduled','fired','cancelled')),
    linked_subscription_id TEXT,
    created_at             TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS ix_reminders_user ON reminders(user_id);

CREATE TABLE IF NOT EXISTS savings_goals (
    id          TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    target_fen  BIGINT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS ix_savings_goals_user ON savings_goals(user_id);

CREATE TABLE IF NOT EXISTS reservations (
    id                TEXT PRIMARY KEY,
    goal_id           TEXT NOT NULL REFERENCES savings_goals(id) ON DELETE CASCADE,
    amount_fen        BIGINT NOT NULL,
    state             TEXT NOT NULL DEFAULT 'held'
                        CHECK (state IN ('held','released','consumed')),
    source_account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS cards (
    id                   TEXT PRIMARY KEY,
    user_id              TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    display_name         TEXT NOT NULL,
    masked_number        TEXT NOT NULL,
    expiry_month         INTEGER NOT NULL,
    expiry_year          INTEGER NOT NULL,
    status               TEXT NOT NULL DEFAULT 'issued'
                          CHECK (status IN ('issued','active','frozen','lost','cancelled','expired')),
    single_tx_limit_fen  BIGINT,
    monthly_limit_fen    BIGINT,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS ix_cards_user ON cards(user_id);

CREATE TABLE IF NOT EXISTS wealth_products (
    id                            TEXT PRIMARY KEY,
    name                          TEXT NOT NULL,
    category                      TEXT NOT NULL
                                    CHECK (category IN ('money_market','bond','fund','simulation_only')),
    risk_level                    TEXT NOT NULL CHECK (risk_level IN ('low','medium','high')),
    expected_annual_return_pct    DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    min_subscription_fen          BIGINT NOT NULL,
    suitability_required          TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS wealth_holdings (
    id             TEXT PRIMARY KEY,
    user_id        TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    product_id     TEXT NOT NULL REFERENCES wealth_products(id) ON DELETE CASCADE,
    amount_fen     BIGINT NOT NULL,
    subscribed_at  TIMESTAMPTZ NOT NULL,
    status         TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','redeemed')),
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS ix_wealth_holdings_user ON wealth_holdings(user_id);

CREATE TABLE IF NOT EXISTS workflows (
    id               TEXT PRIMARY KEY,
    user_id          TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    request_id       TEXT NOT NULL,
    plan_version     INTEGER NOT NULL DEFAULT 1,
    status           TEXT NOT NULL DEFAULT 'draft'
                      CHECK (status IN ('draft','awaiting_confirmation','ready','running',
                                         'paused','completed','failed','cancelled','compensation_required')),
    interrupt_reason TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS ix_workflows_user ON workflows(user_id);

CREATE TABLE IF NOT EXISTS workflow_nodes (
    id                   TEXT PRIMARY KEY,
    workflow_id          TEXT NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    node_index           INTEGER NOT NULL,
    tool                 TEXT NOT NULL,
    parameters           TEXT NOT NULL DEFAULT '{}',
    dependencies         TEXT NOT NULL DEFAULT '[]',
    risk_tier            TEXT NOT NULL DEFAULT 'Green',
    approval_required    BOOLEAN NOT NULL DEFAULT FALSE,
    idempotency_key      TEXT NOT NULL UNIQUE,
    status               TEXT NOT NULL DEFAULT 'pending'
                          CHECK (status IN ('pending','awaiting_confirmation','awaiting_mfa',
                                            'running','completed','failed','paused','compensated','skipped')),
    result               TEXT,
    error                TEXT,
    compensating_node_id TEXT,
    pending_action_id    TEXT,
    UNIQUE (workflow_id, node_index)
);
CREATE INDEX IF NOT EXISTS ix_workflow_nodes_workflow ON workflow_nodes(workflow_id);

CREATE TABLE IF NOT EXISTS pending_actions (
    id                  TEXT PRIMARY KEY,
    user_id             TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tool                TEXT NOT NULL,
    parameters          TEXT NOT NULL DEFAULT '{}',
    parameter_hash      TEXT NOT NULL,
    risk_tier           TEXT NOT NULL CHECK (risk_tier IN ('Green','Yellow','Red')),
    expires_at          TIMESTAMPTZ NOT NULL,
    confirmation_status TEXT NOT NULL DEFAULT 'none'
                          CHECK (confirmation_status IN ('none','confirmed','expired','invalidated')),
    mfa_status          TEXT NOT NULL DEFAULT 'none'
                          CHECK (mfa_status IN ('none','verified','failed','expired')),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    workflow_node_id    TEXT,
    idempotency_key     TEXT
);
CREATE INDEX IF NOT EXISTS ix_pending_actions_user ON pending_actions(user_id);
CREATE UNIQUE INDEX IF NOT EXISTS uq_pending_actions_idem ON pending_actions(idempotency_key)
    WHERE idempotency_key IS NOT NULL;

CREATE TABLE IF NOT EXISTS audit_events (
    id                  TEXT PRIMARY KEY,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    actor_user_id       TEXT,
    session_id          TEXT,
    request_id          TEXT,
    workflow_id         TEXT,
    node_id             TEXT,
    action              TEXT NOT NULL,
    tool                TEXT,
    parameters_masked   TEXT NOT NULL DEFAULT '{}',
    risk_tier           TEXT,
    policy_reason       TEXT,
    confirmation_status TEXT,
    mfa_status          TEXT,
    result              TEXT NOT NULL DEFAULT 'ok' CHECK (result IN ('ok','error','denied','blocked')),
    error_code          TEXT,
    prev_hash           TEXT NOT NULL DEFAULT '',
    hash                TEXT NOT NULL UNIQUE
);
CREATE INDEX IF NOT EXISTS ix_audit_created ON audit_events(created_at);
CREATE INDEX IF NOT EXISTS ix_audit_actor_created ON audit_events(actor_user_id, created_at);

CREATE TABLE IF NOT EXISTS safety_events (
    id         TEXT PRIMARY KEY,
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind       TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    metadata   TEXT NOT NULL DEFAULT '{}'
);
CREATE INDEX IF NOT EXISTS ix_safety_events_user ON safety_events(user_id);
