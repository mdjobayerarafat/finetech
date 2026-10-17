-- BankPilot smart-transfers Phase 6 — split-bills + payee enhancements.
--
-- Adds:
--   - split_bills: a collection request created by one user, listing N
--     payee-shares that other participants can pay in one click.
--   - split_bill_parts: the line items (who owes how much, which payee
--     they would use, whether it's been paid).
--
-- Notes:
--   * Money is integer minor units (fen). No f64 anywhere.
--   * The `parts.payee_id` may be NULL for a guest who hasn't been
--     onboarded yet — the UI then falls back to "type to add".
--   * `created_by` is the user who set up the collection (the host).
--   * `creator_payee_id` lets the host pull funds back to their own
--     account when all parts settle.
--
-- This migration is additive and safe to apply on top of the v1 schema.

CREATE TABLE IF NOT EXISTS split_bills (
    id                    TEXT PRIMARY KEY,
    owner_user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title                 TEXT NOT NULL,
    memo                  TEXT NOT NULL DEFAULT '',
    total_fen             BIGINT NOT NULL CHECK (total_fen > 0),
    currency              TEXT NOT NULL DEFAULT 'CNY',
    source_account_id     TEXT REFERENCES accounts(id) ON DELETE SET NULL,
    status                TEXT NOT NULL DEFAULT 'open'
                          CHECK (status IN ('open','partial','settled','cancelled')),
    due_at                TIMESTAMPTZ,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS ix_split_bills_owner ON split_bills(owner_user_id);
CREATE INDEX IF NOT EXISTS ix_split_bills_status ON split_bills(status);

CREATE TABLE IF NOT EXISTS split_bill_parts (
    id                    TEXT PRIMARY KEY,
    split_bill_id         TEXT NOT NULL REFERENCES split_bills(id) ON DELETE CASCADE,
    -- Either a known payee id, or just a display name for guests.
    payee_id              TEXT REFERENCES payees(id) ON DELETE SET NULL,
    participant_name      TEXT NOT NULL,
    participant_phone_tail TEXT,
    share_fen             BIGINT NOT NULL CHECK (share_fen >= 0),
    status                TEXT NOT NULL DEFAULT 'unpaid'
                          CHECK (status IN ('unpaid','paid','declined')),
    paid_at               TIMESTAMPTZ,
    paid_transfer_id      TEXT REFERENCES transfers(id) ON DELETE SET NULL,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS ix_split_bill_parts_bill ON split_bill_parts(split_bill_id);
CREATE INDEX IF NOT EXISTS ix_split_bill_parts_status ON split_bill_parts(status);

-- Helpful trigger: when a part flips to paid, recompute the parent
-- split_bill status so the UI can filter "what's still owed" cheaply.
CREATE OR REPLACE FUNCTION split_bills_recompute_status() RETURNS trigger AS $$
DECLARE
    remaining_unpaid INTEGER;
    total_parts      INTEGER;
BEGIN
    SELECT COUNT(*) INTO total_parts FROM split_bill_parts WHERE split_bill_id = NEW.split_bill_id;
    SELECT COUNT(*) INTO remaining_unpaid
        FROM split_bill_parts
        WHERE split_bill_id = NEW.split_bill_id AND status <> 'paid';
    UPDATE split_bills
       SET status = CASE
           WHEN remaining_unpaid = 0 AND total_parts > 0 THEN 'settled'
           WHEN remaining_unpaid < total_parts THEN 'partial'
           ELSE status
       END,
       updated_at = NOW()
     WHERE id = NEW.split_bill_id;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_split_bill_parts_recompute ON split_bill_parts;
CREATE TRIGGER trg_split_bill_parts_recompute
    AFTER UPDATE OF status ON split_bill_parts
    FOR EACH ROW EXECUTE FUNCTION split_bills_recompute_status();