---
id: llm-safety
title: "3 · LLM safety boundaries"
sidebar_label: "3 · LLM safety"
sidebar_position: 4
---

# 3. LLM safety boundaries

> 中文: [03-llm-safety.md (中文)](./zh/03-llm-safety.md)

The LLM is a planner, not an authority. The non-negotiable
architectural rules are:

1. **Structured output only.** The LLM emits `proposed_actions[]` —
   structured tool calls with typed arguments. It does not emit
   money, balances, or "completed" status.
2. **Server-side schema validation.** Every tool call is
   re-validated against the `ToolSpec::arguments_schema` (JSON
   Schema). Missing or extra fields → the action is rejected, not
   silently coerced.
3. **Recipient re-resolution.** Every recipient, account, and
   merchant is re-resolved against the database inside the tool.
   The LLM's `payee_id` or `account_id` is never trusted directly
   — it's looked up by phone-tail, display name, or alias.
4. **No secret leakage.** The LLM never sees the user's password,
   OTP, or full account number. It can ask the user for
   confirmation but cannot bypass it.
5. **Tool allowlist.** Only the canonical names in
   `tools::registry()` are dispatchable. Anything else is dropped
   silently. Admin `forbid_tool` rules further narrow the set per
   user.
6. **Fixed system prompt.** The system prompt is fixed and
   versioned. The admin "steering prefix" is the only runtime
   injection point, and it is **append-only** (it cannot remove
   the system rules).

These are enforced by tests:

- `tests/security_*.rs` — direct integration tests for the policy
  and the planner.
- `redteam.test` tool — exercises the LLM end-to-end with crafted
  prompts (see [§2.3](./02-audit.md#23-red-team--prompt-injection-suite)).

**Next:** [§4 — Known risks and the residual attack surface →](./04-known-risks.md)
