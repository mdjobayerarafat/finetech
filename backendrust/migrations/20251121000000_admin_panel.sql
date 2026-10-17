-- BankPilot admin panel — agent rules engine + admin monitoring indexes.
--
-- This migration introduces a *rules engine* that lets admins constrain what
-- the agent can do at runtime:
--
--   * `forbid_tool` rules remove a tool from the planner's allowlist for any
--     user. They are hard denials — even a multi-step workflow cannot use a
--     forbidden tool. The audit log records each denial.
--   * `restrict_payee` rules block transfers to specific payees by name
--     substring (e.g. "test merchant", "scammer@x") or phone tail.
--   * `restrict_memo` rules block memos whose text matches a pattern
--     (regex substring, e.g. "bitcoin", "wire only").
--   * `restrict_amount_fen` rules cap a single transfer amount.
--   * `restrict_daily_fen` rules cap a user's total daily transfer volume.
--   * `system_prompt_prefix` rules inject an additional paragraph into the
--     LLM system prompt for the next turn(s). Useful for
--     "during the demo week, never suggest investment X" or similar.
--
-- Rules are evaluated at *planner* time (before the LLM call) so the
-- restrictions are enforced even if the LLM is tricked. Every match is
-- written to `audit_events` with `action = 'agent_rule.match'` so the admin
-- console can show "this rule fired N times today".
--
-- In addition we add a few indexes for admin monitoring (audit / safety /
-- conversations) — these pay off as the demo scales up.

-- --- Agent rules engine ---------------------------------------------------

CREATE TABLE IF NOT EXISTS agent_rules (
    id              TEXT PRIMARY KEY,
    kind            TEXT NOT NULL
                    CHECK (kind IN (
                        'forbid_tool',
                        'restrict_payee',
                        'restrict_memo',
                        'restrict_amount_fen',
                        'restrict_daily_fen',
                        'system_prompt_prefix'
                    )),
    name            TEXT NOT NULL,                 -- short admin label
    pattern         TEXT NOT NULL,                 -- regex substring / payee substring / tool name / amount fen
    severity        TEXT NOT NULL DEFAULT 'block'  -- 'block' | 'warn' | 'log'
                    CHECK (severity IN ('block','warn','log')),
    applies_to      TEXT NOT NULL DEFAULT 'all',   -- 'all' | user_id
    enabled         BOOLEAN NOT NULL DEFAULT TRUE,
    notes           TEXT,
    created_by      TEXT REFERENCES users(id) ON DELETE SET NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS ix_agent_rules_enabled ON agent_rules(enabled, kind);
CREATE INDEX IF NOT EXISTS ix_agent_rules_applies ON agent_rules(applies_to);

-- Auto-bump updated_at on UPDATE.
CREATE OR REPLACE FUNCTION agent_rules_touch() RETURNS trigger AS $$
BEGIN
    NEW.updated_at := NOW();
    RETURN NEW;
END
$$ LANGUAGE plpgsql;
DROP TRIGGER IF EXISTS trg_agent_rules_touch ON agent_rules;
CREATE TRIGGER trg_agent_rules_touch
    BEFORE UPDATE ON agent_rules
    FOR EACH ROW EXECUTE FUNCTION agent_rules_touch();

-- --- Admin monitoring indexes --------------------------------------------
-- The admin overview / audit / safety pages filter by user and sort by
-- time. These composite indexes keep those queries cheap as the dataset
-- grows.

CREATE INDEX IF NOT EXISTS ix_audit_events_user_time
    ON audit_events(actor_user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS ix_audit_events_action_time
    ON audit_events(action, created_at DESC);
CREATE INDEX IF NOT EXISTS ix_safety_events_user_time
    ON safety_events(user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS ix_safety_events_kind_time
    ON safety_events(kind, created_at DESC);
CREATE INDEX IF NOT EXISTS ix_conversation_messages_user_time
    ON conversation_messages(user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS ix_workflows_user_time
    ON workflows(user_id, created_at DESC);

-- --- Seed a sensible baseline --------------------------------------------
-- We pre-populate a couple of rules so the admin panel is not empty on a
-- fresh seed. Admins can delete or edit any of these from the UI.
INSERT INTO agent_rules (id, kind, name, pattern, severity, applies_to, enabled, notes, created_by)
SELECT 'rule_seed_deny_wire_only', 'restrict_memo', 'Block "wire only" memos',
       'wire only', 'block', 'all', TRUE,
       'Common social-engineering phrase; the demo seeds one to show the matcher.',
       u.id
FROM users u WHERE u.username = 'admin'
ON CONFLICT (id) DO NOTHING;

INSERT INTO agent_rules (id, kind, name, pattern, severity, applies_to, enabled, notes, created_by)
SELECT 'rule_seed_cap_demo_daily', 'restrict_daily_fen', 'Cap demo customers at ¥50,000/day',
       '5000000', 'block', 'all', TRUE,
       'Defence-in-depth on top of config.daily_limit_fen — surfaces in the admin audit when hit.',
       u.id
FROM users u WHERE u.username = 'admin'
ON CONFLICT (id) DO NOTHING;

INSERT INTO agent_rules (id, kind, name, pattern, severity, applies_to, enabled, notes, created_by)
SELECT 'rule_seed_no_voice_tts_for_xfer', 'system_prompt_prefix', 'Advisory: avoid TTS for transfer confirmations',
       'When proposing a money movement, do not also emit a voice.tts call in the same turn. The confirmation step already speaks the amount; double-speaking is noisy.',
       'warn', 'all', TRUE,
       'Soft rule (severity=warn) — added to the LLM system prompt as a hint, not a hard block.',
       u.id
FROM users u WHERE u.username = 'admin'
ON CONFLICT (id) DO NOTHING;
