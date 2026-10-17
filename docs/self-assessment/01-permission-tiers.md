---
id: permission-tiers
title: "1 · Permission tiers"
sidebar_label: "1 · Permission tiers"
sidebar_position: 2
---

# 1. Permission tiers

> 中文: [01-permission-tiers.md (中文)](./zh/01-permission-tiers.md)

## 1.1 The three tiers

Every mutating action the agent can take is classified into one of
three risk tiers, defined as constants in
`backendrust/src/policy.rs`:

| Tier | Constant | Meaning |
|---|---|---|
| **Green** | `policy::GREEN` | Read-only or trivially reversible. Auto-approved after schema + argument validation. |
| **Yellow** | `policy::YELLOW` | Reversible but consequential. Auto-approved, surfaced clearly in the UI. |
| **Red** | `policy::RED` | Irreversible or money-moving. Always requires explicit human confirmation; MFA when configured. |

The decision is **monotonic in a single conversation**: a node can
escalate from Yellow to Red if its arguments trip a higher-risk
rule, but it cannot downgrade. This is enforced inside
`planner::decide_node` — there is no `tier: downgraded` codepath.

## 1.2 How a tier is decided

Tier assignment is split across three files, in this order of
authority:

1. **`policy::decide_*` functions** (`src/policy.rs`) — the canonical
   source. Each mutating tool has a decision function that returns a
   `Decision { tier, reason, escalate_from, flags }`. Examples:
   - `decide_transfer` inspects amount, payee verification status,
     destination account, daily-cumulative totals, and recent
     high-risk behavior. It can escalate from Yellow to Red when the
     daily threshold is crossed or after repeated rejected attempts.
   - `decide_card_action("freeze")` → Yellow;
     `decide_card_action("cancel")` → Yellow but with
     `escalate_from: None` and a `confirm_required` flag.
   - `decide_goal_reservation()` → Yellow; new
     `decide_budget_reserve()` → Red (because it actually moves
     `available_balance_fen` → `reserved_balance_fen`).
2. **Per-tool spec in `src/tool_spec.rs`** — declares the
   `default_tier` for tools that don't have a specialized decision
   function (e.g. `reminders.create` → Green,
   `subscriptions.cancel` → Yellow).
3. **Admin-configured `agent_rules` in Postgres**
   (`agent_rules::forbidden_tools`, `restrict_amount_fen`,
   `restrict_daily_fen`) — these are evaluated *after* the policy
   decision. A `forbid_tool` rule blocks the tool outright; a
   `restrict_daily_fen` rule re-evaluates the Red/Yellow line
   against the user's rolling 24-hour sum, with `SELECT ... FOR
   UPDATE` on the account row so two parallel requests cannot both
   pass.

The planner's `build_plan` is the only place that calls these. There
is no way to bypass `decide_node` from the agent route.

## 1.3 Per-tool tier table (current)

| Canonical tool | Default tier | Notes |
|---|---|---|
| `accounts.list`, `accounts.balance_summary`, `accounts.balance_check` | Green | Read-only. |
| `transactions.recent`, `spending.report`, `payees.list`, `cards.list`, `subscriptions.list`, `wealth.summary`, `user.profile`, `forecast.simulate` | Green | Read-only. |
| `budgets.list` | Green | Read-only. |
| `reminders.create` | Green | Notification only. |
| `subscriptions.cancel`, `subscriptions.remind` | Yellow | Reversible; merchant-side action. |
| `goals.create_reservation` | Yellow | Money held, not moved. |
| `budgets.set` | Yellow | Soft cap; no money movement. |
| `cards.issue`, `cards.action` (freeze / unfreeze / update_limits) | Yellow | Reversible. |
| `wealth.subscribe` | Yellow | Subject to `decide_wealth_subscribe` suitability check. |
| `wealth.redeem` | Yellow | Subject to holding-state checks. |
| `goals.consume_reservation`, `goals.release_reservation` | Yellow | Reservation settlement. |
| `budgets.reserve` | **Red** | Moves `available → reserved`. The "spouse's birthday, reserve ¥1000" cross-scenario flow lands here. |
| `transfer.create_pending` | **Yellow or Red** | `decide_transfer` re-evaluates. Red when: amount > per-tx threshold, unverified payee, daily-cumulative cap reached, repeated recent safety events, or `forbid_tool`/`restrict_daily_fen` rule trips. |
| `split_bill.create` | Yellow | Idempotent, fully reversible. |
| `connectors.merchant.order` | Yellow | Bounded by a per-merchant amount cap. |
| `fraud.score`, `redteam.test`, `voice.tts` | Green | Diagnostic / output. |

## 1.4 What "Yellow" means at runtime

A Yellow-tier node is **not** silently executed. The agent route:

1. Returns a `pending_action` object with `risk_tier: "Yellow"`,
   `expires_at`, and an immutable `parameter_hash` (SHA-256 of the
   canonical JSON).
2. The UI renders the `ConfirmationCard` with a clearly labeled
   Yellow badge, a one-line summary of the action, and a single
   "Confirm" / "Cancel" button pair.
3. On confirm, the backend re-validates the *exact* argument hash.
   If the user (or an attacker) tampered with `pending_actions`
   between proposal and confirm, the confirm returns
   `parameter_hash_mismatch` and the action is not executed.
4. The action runs exactly once. A duplicate `confirm` is a no-op
   that returns the cached result.

## 1.5 What "Red" means at runtime

A Red-tier node adds a step between confirm and execute:

1. After `parameter_hash` re-check, the route calls
   `actions::verify_action` which checks the `MFA_DEMO_CODE` (in
   this demo) — three failed attempts mark the user as
   `safety_lock_state = "locked"`.
2. `actions::confirm_action` then runs the tool under a transaction
   and writes to the audit log.
3. If `forbid_tool` or `restrict_daily_fen` rules are active, the
   confirm path returns a `forbidden` or `daily_limit_exceeded`
   error and the user is shown an explanation, not a generic
   failure.

A Red action that *fails mid-execution* is compensated when
possible. For example, a `transfer.create_pending` whose downstream
payee lookup fails after MFA has its reservation released. The
`compensation` field on each `PlannedNode` is the canonical
release tool.

## 1.6 Session, role, and CSRF

| Concern | Where it's enforced |
|---|---|
| Authentication | `routes::auth::login` issues a session cookie (`HttpOnly`, `SameSite=Strict`). The cookie is the only auth credential. |
| Authorization | Every protected route resolves the `AuthUser` extractor; `role == "admin"` is required for `/api/v1/admin/*`. No frontend-only role check. |
| CSRF | A CSRF token is bound to the session and required as `X-CSRF-Token` on every state-changing request. The frontend reads it from `/api/v1/auth/me` and the axios/fetch wrapper attaches it automatically. |
| Safety lock | `safety::assert_unlocked(user)` runs at the top of every mutating route. Three failed MFA attempts within a window set `safety_lock_state = "locked"`. |
| Daily limit | `restrict_daily_fen` rules are evaluated inside `planner::build_plan` *and* again at confirm time, with the account row locked. |

**Next:** [§2 — Audit and observability →](./02-audit.md)
