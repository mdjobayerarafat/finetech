---
id: banking-api-safety
title: "7 · Banking & API safety"
sidebar_label: "7 · Banking / API safety"
sidebar_position: 8
---

# 7. Banking & API safety

> 中文: [07-banking-api-safety.md (中文)](./zh/07-banking-api-safety.md)

`§3` covered LLM / agent attacks. This page is the **banking and
HTTP API** counterpart — twelve named attack classes from the OWASP
API / financial-services playbook, how each one would hit FinTech Pioneers,
and the concrete defence that stops it. The same honesty rule
applies: every *Defence* row points to code, every gap is named in
[§4](./04-known-risks.md).

## 7.1 Authentication & session

| # | Attack | How it hits FinTech Pioneers | Defence | Where |
|---|---|---|---|---|
| 1 | **Broken auth — weak JWT, no expiry, token reuse** | A demo-grade auth that ships `alg: none` or a never-expiring token gives an attacker a permanent session the first time a session cookie leaks (XSS, log dump, mis-set cookie). | We do **not** use JWTs. Sessions are server-side rows in `sessions` with `expires_at` (default 12h, env-driven `SESSION_TTL_SECONDS`). The cookie is opaque (`Cookie::http_only(true)`, `SameSite::Lax`, `Secure` in production). The CSRF token is a per-session random stored alongside it. There is no "alg: none" footgun because there is no signing key to forget. | `sessions.rs::create_session`, `sessions.rs::cookie_attrs` |
| 2 | **Stale session after privilege change** | A session issued before a role / lockout / MFA-fail state change keeps the old privileges. | **Gap (R-7).** Sessions are not currently re-issued on `role`, `safety_lock_state`, or `password_hash` change. The hardening path is in `04-known-risks.md` (R-7). For the demo the only user whose state changes during a run is `locked_vip`, and that user is locked at seed time, so the stale-session window is bounded. | `safety.rs`, `R-7 in §4` |
| 3 | **Brute force / enumeration** — login, OTP, payee lookup | A scripted attacker tries the 100 most common passwords, or walks phone-tails 0000…9999 to enumerate payees. | (a) **MFA failure** is rate-limited at the *user* level: `MAX_FAILED_MFA=3` consecutive wrong codes triggers `safety_lock_state = locked` and an audit event. (b) **Login** has no per-IP rate limit (R-6 lists this as a hardening path); the demo relies on a small user base + Argon2id password hashing. (c) **Payee lookup** returns a *generic* `not_found` for both unknown phone tails and unknown IDs — no enumeration oracle. | `safety.rs::record_failed_mfa`, `errors.rs::not_found`, R-6 in §4 |

## 7.2 Authorisation

| # | Attack | How it hits FinTech Pioneers | Defence | Where |
|---|---|---|---|---|
| 4 | **IDOR / BOLA** — change `account_id` in a request to read someone else's data | The single biggest category of API breach. Any endpoint that takes a user-controlled id and returns data is a candidate. | **Ownership is re-checked on every row** in the tool layer, not at login. Every write tool that takes an `id` does `if row.user_id != ctx.user.id { return ToolError::NotFound(...) }` before doing anything else. The same `user_id` predicate is in every read SQL (`WHERE a.user_id = $1`). The cross-user test in the suite asserts that user A asking for user B's account id returns 404, not 200. | `tools.rs::t_*`, `cross_user.rs` |
| 5 | **Confirmation spoofing (UI-redress)** — model text says "you'll pay Alice", confirmation card says "you'll pay Eve" | If the UI renders the confirmation from model prose, the model and the action can disagree and the user confirms something different. | `ConfirmationCard` is rendered from the **typed `pending_action`**, not from any model field. The card reads `pending_action.tool` and the matching `plan.nodes[…].parameters` — both are server-validated, immutable, and pre-confirmation locked. The model's narrative is shown above the card as decoration, never as the source of truth. The CSRF token is bound to the *pending_action_id*, so a stolen / replayed card cannot re-fire. | `frontend/src/components/confirmation-card`, `routes/actions.rs::confirm_action` |

## 7.3 Transaction integrity

| # | Attack | How it hits FinTech Pioneers | Defence | Where |
|---|---|---|---|---|
| 6 | **Replay / double-spend** — resubmit a confirmed transfer | The network hiccups, the user double-clicks, a buggy client retries — same transfer settles twice. | (a) `t_transfer_create_pending` **requires** an `idempotency_key`. The same key + same parameters returns the existing `pending_action`; the same key + *different* parameters returns `409 conflict`. (b) `pending_actions` has a `confirmation_status` lifecycle (`pending → confirmed → executed`); once `executed`, re-confirming returns `409 already_executed`. (c) `expires_at` (default 5 min) makes any unconfirmed action unreplayable after the window. | `tools.rs::t_transfer_create_pending`, `routes/actions.rs::confirm_action` |
| 7 | **Race conditions** — two concurrent transfers overdrawing an account | Two `POST /transfers` land at the same time; both see the same balance, both pass the `>= amount` check, both succeed. | Every write tool opens a `sqlx` transaction and calls `lock_account(&mut *tx, account_id)`, which issues `SELECT … FROM accounts WHERE id = $1 AND user_id = $2 FOR UPDATE`. The second transfer blocks until the first commits, then re-reads the *new* balance and fails the available-balance check. The tool then `commit`s — no partial state if anything in between errors. | `tools.rs::lock_account` |
| 8 | **Business-logic abuse** — split a large transfer to dodge limits (structuring); probe for valid payees | An attacker uses 9 × ¥999 transfers instead of 1 × ¥9,000 to bypass a single-tx cap; or hammers `/payees?phone_tail=…` to enumerate. | (a) **`restrict_daily_fen` rule** in `agent_rules.rs` caps the *cumulative* amount per account over the rolling window (default `DAILY_LIMIT_FEN = 1_000_000` fen = ¥10,000). The planner refuses any node whose `today_fen + attempted_fen > cap`, not just `attempted_fen > cap`. The explainer shows both numbers. (b) **Single-tx `restrict_amount_fen`** catches the upper-bound side. (c) **Payee lookup** returns the same generic `not_found` for unknown and known-but-not-yours; an attacker can't distinguish "no such payee" from "not your payee". (d) **Per-user velocity counters** are a hardening path (R-6), not in code yet. | `agent_rules.rs`, `policy_explainer.rs`, R-6 in §4 |
| 9 | **MFA fatigue / bypass** — spam the user with prompts until they approve; or skip MFA via another endpoint | A compromised chat session repeatedly proposes a Red-tier action; or a Red-tier tool is exposed without the MFA gate. | (a) **`MAX_FAILED_MFA = 3`** locks the account on the third wrong code. (b) **MFA is bound to the action**: the `verify_action` endpoint requires `pending_action_id` and `mfa_code` together; codes are not reusable across actions. (c) **MFA is server-enforced on every Red node** in `routes/actions.rs::confirm_action` — there is no Red-tier execute path that bypasses it. (d) The LLM cannot mark an action Green or Yellow; tier is decided in `policy.rs` only. (e) **MFA fatigue (many prompts to one user)** is a hardening path: future work is a per-user `mfa_prompt` rate counter independent of the failed-attempt counter. | `safety.rs`, `routes/actions.rs::verify_action`, `policy.rs` |

## 7.4 Input / output handling

| # | Attack | How it hits FinTech Pioneers | Defence | Where |
|---|---|---|---|---|
| 10 | **SQL injection** | A field the user controls (payee name, memo, search) gets concatenated into a SQL string. | (a) **Every query is parameterised via `sqlx::query` + `.bind`** — `sqlx` macros refuse to compile untyped string interpolation. (b) A repo-wide grep (`! grep -RE 'query!\([^)]*\+' src/`) is the regression net; any `format!` inside a `query!` macro fails CI. (c) `ORDER BY` and other SQL fragments that can't take bind parameters are restricted to a hard-coded allowlist in the few tools that accept them (e.g. `transactions.list` only takes a fixed set of order keys). | `tools.rs::t_*`, `transactions.rs`, grep guard in `tests/` |
| 11 | **XSS in chat and notifications** | Model output or merchant text rendered as HTML. A single `<img onerror=…>` in a memo steals the session cookie. | (a) **No `dangerouslySetInnerHTML` anywhere** in the chat / receipt / notification paths (a repo-wide grep is the test). (b) **All user- and model-supplied text** is rendered as React text, which escapes by default. (c) **A strict CSP is *not* currently set** (R-8 lists this as a hardening path); the demo relies on React's escaping. (d) Markdown rendering, when added for receipts, must go through a sanitiser; the audit log's `parameters_masked` already does the equivalent for stored model input. | `frontend/src/`, R-8 in §4, `audit.rs::mask_parameters` |
| 12 | **SSRF / unsafe connectors** | A merchant- or receipt-connector tool fetches a URL the user (or the model) supplied. An attacker uses `http://10.0.0.5/…` to probe the internal network. | (a) **No tool takes a user-controlled URL.** The receipt upload goes through a typed connector (`connectors.merchant.order`) that only accepts an internal merchant id, resolved server-side. (b) **Any future URL fetch** must go through a single `http_client::safe_get(url)` wrapper that resolves DNS, blocks RFC1918 / link-local / loopback / `*.internal`, and only allows `https://` to a curated allowlist. (c) The current connector set is small enough that a code-review checklist covers it; the wrapper is the hardening path. | `routes/agent.rs::configure`, R-3 / R-15 in §4 |

## 7.5 Supply chain, secrets, observability

| # | Attack | How it hits FinTech Pioneers | Defence | Where |
|---|---|---|---|---|
| 13 | **Secrets leakage** | API keys, the Postgres URL, or the MFA demo code end up in git, in a log line, or in an LLM prompt. | (a) **`.env` is in `.gitignore`**; the seed file uses `'$argon2id$…'` placeholders. (b) **The LLM prompt is built from a fixed allowlist of fields** (`accounts.balance_check`, `accounts.list`, etc.) — the user's password, OTP, full account number, MFA code, and `OPENAI_API_KEY` are never interpolated into a prompt. (c) **Log lines are filtered** for OTP-shaped 6-digit codes, full PAN-shaped 16–19 digit numbers, and `sk-…` API keys before write. (d) **The audit chain** only stores `parameters_masked`, with phone-tails and amounts that match the display contract. | `llm_http.rs` (allowlist), audit masker, `.gitignore` |
| 14 | **PII in logs / audit** | LLM prompts, full account numbers, or raw memos are persisted in `llm_calls` or `audit_events` and surfaced to an admin. | (a) **`audit_events.parameters_masked`** strips known-PII fields before insert; the un-masked parameters object never reaches the database. (b) **`llm_calls` retains a sanitised prompt** — the *raw* user message is stored with PII patterns (phone tails, full card numbers) replaced by placeholders. (c) **Retention**: the audit table is unbounded in the demo (R-11); the hardening path is a monthly partition + cold-storage archive. (d) Admin endpoints return the masked form; the only raw view is the audit-tamper test (`verify_chain`) which compares hashes, not contents. | `audit.rs::mask_parameters`, `llm_audit.rs`, R-11 in §4 |
| 15 | **Insider / admin abuse** — an admin edits history, or sees another user's data | A malicious operator with DB write access edits `audit_events`; a malicious admin calls `/api/v1/admin/users` and exfiltrates. | (a) **`audit_events` is append-only in code** — the grep test `! grep -RE 'UPDATE audit_events' src/` fails CI if any writer is added; `audit::append_event` is the single insertion point. (b) **Hash chain** detects post-hoc tampering: `prev_hash || canonical_json → sha256 → hash`, and `verify_chain` walks the table. (c) **Role separation** is in `AuthUser`: `admin` role is required on `/api/v1/admin/*`; a customer-role session gets `403 forbidden_role` even with a valid session cookie. (d) **Database-level guards** (revoking UPDATE / DELETE on the audit table, triggers) are a hardening path (R-12). | `audit.rs`, `routes/admin.rs`, R-12 in §4 |

## 7.6 How to read this matrix

- The **Defence** column again points to *code*. If a function
  doesn't exist, the row is a hardening path — link it to the
  matching `R-*` in [§4](./04-known-risks.md).
- Two of the rows (1, 4, 6, 7, 9, 10, 13) are **live in code
  today**; they are the non-negotiable banking invariants.
- Six of the rows (2, 3, 8 partial, 11 partial, 12, 14 retention,
  15 DB-level) are **partially live** with the residual named in
  §4. The demo's "we ship the design" stance is fine for the
  submission; the production-hardening path is concrete.
- The matrix is also a **regression target**: each row has a
  corresponding test (most in `backendrust/tests/`, the SQL-injection
  guard is a grep test, the audit-chain guard is `audit_chain_integrity`).

**Back to:** [Overview →](./README.md) · [Next: §8 — (reserved) →](#)
