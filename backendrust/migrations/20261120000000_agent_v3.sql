-- BankPilot v3 — close-the-loop features (Layer 1) + proactive intelligence (Layer 3)
-- + better error UX (Layer 5.5).
--
-- This migration is additive and idempotent. All new columns are NULLable so existing
-- rows are not disturbed. New tables are created with IF NOT EXISTS.
--
-- New columns:
--   users.daily_digest_seen_at           — when the user last acknowledged the digest
--   payees.verification_status           — "unverified" | "pending" | "verified"
--   payees.verified_at                   — set when verification completes
--   subscriptions.snooze_until           — NULL or a future timestamp; gates renewals
--   subscriptions.deferred_charge        — JSONB: {amount_fen, was_expected_at, reason}
--   subscriptions.uncancel_until         — NULL or a future timestamp; "uncancel" window
--   transfers.reversed_at                — set when a 5-min reverse is applied
--   transfers.reverse_reason             — user-supplied reason text
--   transactions.dispute_status          — NULL | "draft" | "submitted"
--   transactions.dispute_text            — generated draft body
--   transactions.dispute_generated_at    — timestamp
--
-- New tables:
--   goal_contributions         — explicit ledger of money moved into a goal
--   payee_verifications        — round-trip verification codes for unverified payees
--   daily_digest_state         — last seen digest version per user (so we don't repeat)

-- --- users -------------------------------------------------------------
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS daily_digest_seen_at TIMESTAMPTZ;

-- --- payees ------------------------------------------------------------
ALTER TABLE payees
    ADD COLUMN IF NOT EXISTS verification_status TEXT NOT NULL DEFAULT 'unverified'
        CHECK (verification_status IN ('unverified','pending','verified','failed'));
ALTER TABLE payees
    ADD COLUMN IF NOT EXISTS verified_at TIMESTAMPTZ;
ALTER TABLE payees
    ADD COLUMN IF NOT EXISTS mock_destination_kind TEXT NOT NULL DEFAULT 'bank_account';

-- --- subscriptions -----------------------------------------------------
ALTER TABLE subscriptions
    ADD COLUMN IF NOT EXISTS snooze_until TIMESTAMPTZ;
ALTER TABLE subscriptions
    ADD COLUMN IF NOT EXISTS deferred_charge JSONB;
ALTER TABLE subscriptions
    ADD COLUMN IF NOT EXISTS uncancel_until TIMESTAMPTZ;

-- --- transfers ---------------------------------------------------------
ALTER TABLE transfers
    ADD COLUMN IF NOT EXISTS reversed_at TIMESTAMPTZ;
ALTER TABLE transfers
    ADD COLUMN IF NOT EXISTS reverse_reason TEXT;

-- --- transactions ------------------------------------------------------
ALTER TABLE transactions
    ADD COLUMN IF NOT EXISTS dispute_status TEXT
        CHECK (dispute_status IS NULL OR dispute_status IN ('draft','submitted','resolved','rejected'));
ALTER TABLE transactions
    ADD COLUMN IF NOT EXISTS dispute_text TEXT;
ALTER TABLE transactions
    ADD COLUMN IF NOT EXISTS dispute_generated_at TIMESTAMPTZ;
CREATE INDEX IF NOT EXISTS ix_transactions_merchant
    ON transactions (merchant);
CREATE INDEX IF NOT EXISTS ix_transactions_user_occurred
    ON transactions (occurred_at DESC) WHERE status = 'posted';

-- --- goal_contributions (new) -----------------------------------------
CREATE TABLE IF NOT EXISTS goal_contributions (
    id                 TEXT PRIMARY KEY,
    goal_id            TEXT NOT NULL REFERENCES savings_goals(id) ON DELETE CASCADE,
    user_id            TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    source_account_id  TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    amount_fen         BIGINT NOT NULL CHECK (amount_fen > 0),
    memo               TEXT NOT NULL DEFAULT '',
    transfer_id        TEXT REFERENCES transfers(id) ON DELETE SET NULL,
    source             TEXT NOT NULL DEFAULT 'manual'
                       CHECK (source IN ('manual','rebalance','auto')),
    created_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS ix_goal_contributions_goal ON goal_contributions (goal_id, created_at DESC);
CREATE INDEX IF NOT EXISTS ix_goal_contributions_user ON goal_contributions (user_id, created_at DESC);

-- --- payee_verifications (new) ----------------------------------------
CREATE TABLE IF NOT EXISTS payee_verifications (
    id            TEXT PRIMARY KEY,
    payee_id      TEXT NOT NULL REFERENCES payees(id) ON DELETE CASCADE,
    user_id       TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    challenge_fen BIGINT NOT NULL CHECK (challenge_fen > 0),
    code          TEXT NOT NULL,
    status        TEXT NOT NULL DEFAULT 'pending'
                  CHECK (status IN ('pending','confirmed','expired','failed')),
    expires_at    TIMESTAMPTZ NOT NULL,
    confirmed_at  TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS ix_payee_verifications_payee ON payee_verifications (payee_id, status);
CREATE INDEX IF NOT EXISTS ix_payee_verifications_user  ON payee_verifications (user_id, status);

-- --- daily_digest_state (new) -----------------------------------------
CREATE TABLE IF NOT EXISTS daily_digest_state (
    user_id          TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    last_seen_at     TIMESTAMPTZ,
    last_seen_hash   TEXT,
    muted            BOOLEAN NOT NULL DEFAULT FALSE
);
