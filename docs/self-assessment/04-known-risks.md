---
id: known-risks
title: "4 · Known risks and the residual attack surface"
sidebar_label: "4 · Known risks (R-1…R-15)"
sidebar_position: 5
---

# 4. Known risks and the residual attack surface

> 中文: [04-known-risks.md (中文)](./zh/04-known-risks.md)

This section is the most important for reviewers. It enumerates the
limitations of the current build honestly. Each entry is structured
as:

- **Risk** — what could go wrong.
- **Status** — *demo-acceptable* or *production-unacceptable* (and
  why).
- **Hardening path** — concrete next steps if the platform were to
  carry real money.

## R-1. Mock MFA

**Risk.** MFA verification uses a single hard-coded
`MFA_DEMO_CODE` (default `000000`, configurable via `MFA_DEMO_CODE`
env var). It is in-process, not delivered out of band. An attacker
with access to the backend can read it from memory or environment.

**Status.** Acceptable for a demo. **Not** acceptable for production.

**Hardening path.** Wire to a real OTP provider (TOTP via
authenticated TOTP URI, or SMS via a vendor with a verified
delivery path), with per-user enrollment and per-action
challenge/response. Add replay protection (single-use codes,
timestamp windows).

## R-2. No real key custody

**Risk.** The audit chain's `hash` is a plain SHA-256 of the
previous chain hash concatenated with the row's canonical JSON.
This is integrity, not authenticity — anyone with database write
access can extend the chain, replacing both `prev_hash` and
`hash` consistently. There is no signing key, no HSM, no
KMS-backed signer.

**Status.** Acceptable for a demo where the database is the
attacker too. **Not** acceptable for any trust boundary between
the operator and an external auditor.

**Hardening path.** Sign each row with a per-deployment key
stored in a KMS; sign only the hash, not the row contents
(privacy); publish the public key for offline verification.

## R-3. No real card / bank integration

**Risk.** All "transfers", "cards", and "wealth holdings" are
rows in Postgres. There is no rail integration (no real card
network, no ACH, no FPS, no PSP). The platform can demonstrate
the *control surface* but cannot move real money.

**Status.** Acknowledged. The README calls this out; the demo
banner is "simulation only".

**Hardening path.** Integrate a partner bank via a regulated
channel. The internal tool allowlist is structured so a new
`rail.bank.x.*` namespace can be added without touching the
planner or the policy engine.

## R-4. Prompt-injection residual

**Risk.** The LLM is the front door for the chat experience. The
defenses — fixed system prompt, structured `proposed_actions`
only, server-side argument re-resolution — are layered, but no
prompt-injection defense is perfect. A sufficiently creative
attacker who can plant text in the LLM's context (memo fields,
merchant names, payee aliases) can attempt to coerce an unsafe
proposal.

**Status.** Mitigated but not eliminated. The red-team suite is
the regression net; new injection patterns must be added as
they're discovered.

**Hardening path.** Move the LLM behind a one-shot function-call
interface with a deterministic schema parser; never let the LLM
produce free-form text that's combined with action decisions in
the same response. Add a "second pair of eyes" tier where a
separate model validates the first model's output before
execution for Red-tier actions.

## R-5. Frontend / backend trust boundary

**Risk.** The frontend renders any state the backend returns. A
compromised backend can serve a fake "transfer successful" page.
The audit chain helps, but only post-hoc.

**Status.** Inherent to the architecture — the backend is the
trust root. The frontend is presentation only.

**Hardening path.** Add a `receipt` endpoint per action that
returns the persisted `audit_events` row alongside the result,
so the UI can verify the action actually ran. (The receipt page
already exists; the API is partial.)

## R-6. No rate limiting at the edge

**Risk.** The Rust backend has a 12-worker actix runtime, but
there is no IP-based or user-based rate limiter in front of
`/api/v1/agent/messages`. A motivated attacker can exhaust LLM
credits or starve the DB pool.

**Status.** Acceptable for demo. The DB pool does have a
`Pool::acquire` timeout (10s), so the platform fails closed
rather than crashing.

**Hardening path.** Add a token-bucket rate limiter at the edge
(Envoy / NGINX), keyed on user id and IP, with a separate
bucket for LLM-bound routes.

## R-7. Sessions are not rotated on privilege change

**Risk.** A session cookie issued before a role change (e.g. a
user is later promoted to admin) is not re-issued. If the
attacker stole the old cookie, the privilege change is moot
for them.

**Status.** Untested. The seed user `locked_vip` is the only
one whose role/state changes during a demo.

**Hardening path.** Rotate the session id on any change to
`role`, `safety_lock_state`, or `password_hash`.

## R-8. No CSRF for read endpoints

**Risk.** The CSRF token is required on state-changing
requests. Read endpoints (GET) are safe by HTTP semantics, but
JSON-bodied reads that leak data via side channels (cache, log)
are possible. We don't currently emit any.

**Status.** Acceptable.

**Hardening path.** Add a `Sec-Fetch-Site` / `Origin` allowlist
check on every endpoint, with strict mode for `/api/v1/admin/*`.

## R-9. The "Set a budget" cross-scenario flow is Red-tier

**Risk.** The new `budgets.reserve` tool moves
`available_balance_fen → reserved_balance_fen` and so is Red.
The chat agent's stub can already produce a complete plan
("spouse's birthday, reserve ¥1000 from my checking") that the
planner accepts. If the user has been socially-engineered into
confirming a reservation for a payee they don't know, the
money is held against the goal until released.

**Status.** Designed behavior. The risk is bounded by the amount
(the cap is whatever the LLM extracted; the user confirms the
exact number) and the goal can be released via
`goals.release_reservation` in the same conversation.

**Hardening path.** Add a "name-confirmation" step for any goal
reservation where the label doesn't match a known contact;
require a "release" reason for any reservation older than
30 days.

## R-10. Daily-limit rule uses wall-clock, not calendar

**Risk.** The `restrict_daily_fen` rolling 24-hour window resets
at the user's local midnight only if the user has a session in
that timezone. Otherwise the platform rolls the window
continuously. This is documented but may surprise a judge who
assumes calendar-day semantics.

**Status.** Documented; tests use the rolling window.

**Hardening path.** Add a `daily_window` rule kind with explicit
`tz` and `rollover` semantics.

## R-11. Audit retention is unbounded

**Risk.** `audit_events` has no TTL. A long-running deployment
will grow the table indefinitely and `verify_chain` will get
slower.

**Status.** Acceptable for a demo of this size.

**Hardening path.** Partition by month, archive signed bundles
to cold storage, and add a `verify_chain(from, to)` windowed
variant.

## R-12. The audit table is mutable in SQL (no DB-level guard)

**Risk.** `audit_events` is a normal table. There is no
`REVOKE UPDATE, DELETE` on it, no `BEFORE UPDATE` trigger, and
no `pg_temporal` / `pg_audit` extension wired in. The
immutability is a *codebase* property, not a *database*
property. Anyone with SQL write access can rewrite history.

**Status.** Acknowledged. The grep test
`! grep -RE 'UPDATE audit_events' src/` is the only guard.

**Hardening path.** Create an `audit_writer` role with
INSERT-only privileges on `audit_events`; have the application
connect as that role, not as a superuser. Add a `BEFORE UPDATE
OR DELETE` trigger that raises.

## R-13. The LLM stub is in the repo

**Risk.** `src/llm.rs::stub_propose` exists as a deterministic
fallback when the upstream LLM is unreachable. It includes
parsers for "set a budget", "spouse's birthday", and a small
set of intents. A future contributor could change the stub to
be more permissive without realizing it's a live code path
under network failure.

**Status.** The stub is documented; the comment in `llm.rs` names
it as the failure-mode fallback.

**Hardening path.** Gate the stub on a config flag
(`LLM_ALLOW_STUB=1` in dev, off in prod). Have CI fail if that
flag is set in a prod build.

## R-14. The optimization report reads user transactions directly

**Risk.** The `report_optimization` query in
`routes::accounts.rs` aggregates over `transactions` for the
authenticated user. If a future change accidentally removes
the `WHERE a.user_id = $1` filter, the report would leak
cross-user data.

**Status.** Tested by the cross-user test (`tests/cross_user.rs`).

**Hardening path.** Centralize all user-scoped queries through a
`for_user(user_id)` helper that wraps the SQL; add a codegen /
linter that flags raw `SELECT … FROM transactions` without a
`user_id` predicate.

## R-15. No defense-in-depth against a hostile database admin

**Risk.** The seed file is shipped with admin credentials in
plaintext (`admin` / `admin1234`-ish, see `seed.sql`). The MFA
code is in env. The audit table is mutable. If a DBA is
malicious, the platform is fully exposed.

**Status.** The threat model in
[`../THREAT_MODEL.md`](../THREAT_MODEL.md) explicitly lists
"compromised operator" as out of scope. This doc echoes that.

**Hardening path.** Separate read and write database roles; keep
the audit table on a different database instance that the
application has INSERT-only access to.

---

**Next:** [§5 — What we are not claiming →](./05-non-claims.md)
