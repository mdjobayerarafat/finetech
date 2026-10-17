-- Budgets: per-category monthly spending caps.
--
-- The agent uses these to (a) cap category spend (a "warning" if a transfer
-- would push spend past the cap), (b) surface breach cards in the
-- optimization report, and (c) feed the forecaster with an extra commitment
-- line so "Can I afford X?" answers respect the budget.
--
-- `category` is the same enum-style category used by `transactions.category`
-- (food, transport, shopping, ...). A row per (user, category) — multiple
-- rows for the same category represent overlapping periods (e.g. one-off
-- trip budgets vs. the standing monthly one).
--
-- `period_start` / `period_end` are optional half-open windows. A NULL pair
-- means "rolling 30 days from NOW()" (the default the API exposes).
-- `account_id` is optional; if set, only debits on that account count
-- against the budget (e.g. a food budget for the primary card only).
--
-- A `note` is free text the agent can echo back when referencing the budget
-- in chat or reminders.

CREATE TABLE IF NOT EXISTS budgets (
    id            TEXT PRIMARY KEY,
    user_id       TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    category      TEXT NOT NULL,
    limit_fen     BIGINT NOT NULL CHECK (limit_fen > 0),
    period_start  TIMESTAMPTZ,
    period_end    TIMESTAMPTZ,
    account_id    TEXT REFERENCES accounts(id) ON DELETE SET NULL,
    note          TEXT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_budgets_user_category
    ON budgets (user_id, category);

-- A pre-recorded log of "the agent set aside X for Y in month Z" — used by
-- the cross-scenario orchestration (e.g. "spouse birthday, reserve ¥1000
-- in October"). Each row is one reservation backed by a savings goal so
-- the existing reservation/reconcile machinery can release the hold if the
-- user cancels. We point at savings_goals because the user-visible concept
-- is the same: money earmarked for a future event.
CREATE TABLE IF NOT EXISTS budget_reservations (
    id              TEXT PRIMARY KEY,
    user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    budget_id       TEXT REFERENCES budgets(id) ON DELETE SET NULL,
    goal_id         TEXT REFERENCES savings_goals(id) ON DELETE SET NULL,
    label           TEXT NOT NULL,
    amount_fen      BIGINT NOT NULL CHECK (amount_fen > 0),
    due_at          TIMESTAMPTZ,
    source_account_id TEXT REFERENCES accounts(id) ON DELETE SET NULL,
    state           TEXT NOT NULL DEFAULT 'held',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_budget_reservations_user
    ON budget_reservations (user_id, state);
