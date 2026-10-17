# Architecture

> See [PLAN.md §6](../PLAN.md) for the component map. This doc explains the **request lifecycle**, the **trust boundaries**, and **deployment shape**.

## 1. High-level diagram

```
                ┌──────────────────────────────────────────────┐
                │             Browser (Next.js 14)             │
                │  Chat · Dashboard · Confirm · Timeline · …   │
                └─────────────────────┬────────────────────────┘
                                      │ session cookie + CSRF
                                      ▼
   ┌──────────────────────────────────────────────────────────────────┐
   │                    Rust + Actix Web API                          │
   │                                                                  │
   │  ┌─────────┐  ┌──────────┐  ┌────────────┐  ┌────────────────┐   │
   │  │  auth/  │→ │  input   │→ │  policy/   │→ │  approvals/    │   │
   │  │  role   │  │ validate │  │  tier dec. │  │  pending-store │   │
   │  └─────────┘  └──────────┘  └────────────┘  └────────┬───────┘   │
   │                                                       │           │
   │                       ┌───────────────┐               │           │
   │                       │  workflows/   │◀──────────────┘           │
   │                       │  DAG executor │                           │
   │                       └───────┬───────┘                           │
   │                               │                                   │
   │   ┌─────── domain services (transfers, bills, subs, …) ─────────┐│
   │   │   • re-resolve recipient/amount                              ││
   │   │   • re-check daily limit + balance                           ││
   │   │   • call fixed tool allowlist                                ││
   │   │   • write ledger entries in tx                               ││
   │   └──────────────────────────────────────────────────────────────┘│
   │                               │                                   │
   │                       ┌───────▼───────┐    append-only events     │
   │                       │   audit/      │ ──────────────────────────┼──► PostgreSQL
   │                       │  hash chain   │                           │
   │                       └───────────────┘                           │
   └──────────────────────────────────────────────────────────────────┘
                  │                          │
                  │ JSON tool-call proposal  │ tool-call (read)
                  ▼                          ▼
       ┌──────────────────┐         ┌────────────────────┐
       │  LLM adapter     │         │ Mock connectors    │
       │  openai/* (live) │         │ merchant · calendar│
       │  stub/* (offline)│         │ flowers · cake     │
       └──────────────────┘         └────────────────────┘
```

## 2. Request lifecycle — the four stages

Every agent request goes through four stages, each owned by a different module. No stage can do another stage's job.

### 2.1 Plan (LLM-owned, *advisory*)

- Input: user message + prior conversation + structured user state (balances, payees).
- LLM is asked to return a **structured proposal** (JSON, validated against a schema):
  ```json
  {
    "intent": "transfer",
    "language": "mixed",
    "entities": { "recipient_query": "Li Wei", "amount_cny": "200.00", "memo": "lunch" },
    "clarification_required": false,
    "proposed_actions": [
      { "tool": "transfer.create_pending", "arguments": { "recipient_query": "Li Wei", "amount_cny": "200.00", "memo": "lunch" } }
    ]
  }
  ```
- LLM **does not** choose the risk tier. The tier is decided in stage 3.
- Memos, payee names, and merchant strings from the user are **never** re-injected as instructions into the LLM prompt (see `SECURITY.md §3`).

### 2.2 Validate (backend-owned, *authoritative*)

- JSON schema check on every argument.
- Server-side **re-resolution**: recipient query → internal `payee_id`, amount → integer fen, account → session-scoped lookup, schedule → RFC 3339 + timezone check.
- Currency normalisation, money bounds, negative/zero rejection.
- Identity derived from the session, never the request body.

### 2.3 Authorise (policy engine, *authoritative*)

- Input: validated tool call + current account state + daily total + recipient history + safety-lock state.
- Output: `tier ∈ {Green, Yellow, Red}` + reason.
- The tier can escalate during a conversation; it cannot be downgraded.
- Yellow/Red create a `pending_action` with an **immutable parameter hash** and expiry.
- Any parameter change after that invalidates the approval.

### 2.4 Execute (backend-owned, *authoritative*)

- Green: run immediately, write audit, return result.
- Yellow: run only after `/actions/{id}/confirm` matches the parameter hash.
- Red: run only after `/actions/{id}/verify` returns a fresh mock MFA success.
- Every state-changing tool has an **idempotency key**; replays are no-ops.
- Audit event is appended with previous-event hash → current hash (chain).

## 3. Trust boundaries

| Boundary | What's trusted | What's NOT trusted |
|---|---|---|
| Browser → API | Session cookie, CSRF token, JSON body *shape* | Any field value (names, memos, file content) |
| API → LLM | Conversation summary, structured user state, tool schema | Free-form text from user / connectors |
| LLM → API | `proposed_actions[]` JSON | The LLM's risk assessment, recipient IDs, balances |
| API → connectors | Mock connector endpoint, request body | Connector response text (sandboxed) |
| Anyone → audit log | Read by owner / admin | Writes (append-only; backend only) |

## 4. Concurrency model

- All state-changing writes happen in a single Postgres transaction.
- Daily transfer total is read with `SELECT … FOR UPDATE` on the account row, then updated in the same tx. This blocks split-and-double-submit races.
- DAG nodes run on a worker pool. Each node uses an idempotency key (`workflow_id + node_id + param_hash`) so retry is safe.
- Workflow pause is cooperative: the executor checks the workflow's `status` between nodes; an in-flight node completes its atomic step, then the executor stops scheduling.

## 5. Failure handling

- LLM unavailable → fall back to stub adapter; return a `partial_plan` with `clarification_required: true` where ambiguity exists.
- Tool call fails (e.g. mock merchant timeout) → node moves to `failed`, workflow pauses, user sees a clear error and a "retry" affordance.
- Pending action expires (default 5 min) → mark `expired`, require re-confirmation.
- Audit write fails → roll back the state-changing tx; the action is not recorded as done.
- Safety lock triggers mid-workflow → all in-flight Yellow/Red nodes move to `paused`; Green can continue for read-only side effects.

## 6. Deployment shape

```
┌────────────┐     ┌───────────────────────┐
│  Browser   │────▶│  Frontend (Next.js)   │   :3000
└────────────┘     └───────────┬───────────┘
                               │ reverse proxy (TLS)
                               ▼
                  ┌────────────────────────┐
                  │  Rust API (Actix Web)  │   :8080
                  └─────┬─────────────┬────┘
                        │             │
                        ▼             ▼
              ┌──────────────┐  ┌──────────────────┐
              │  PostgreSQL  │  │  LLM provider    │
              │  (Coolify)   │  │  (OpenAI-compat) │
              └──────────────┘  └──────────────────┘
```

- Postgres is **external** (Coolify), reached via `DATABASE_URL`. No in-repo dockerised DB.
- Frontend talks to the API over the same origin in production, via the reverse proxy.
- LLM adapter is pluggable; the rest of the system never imports a provider SDK directly.

## 7. Where to read next

- Module-by-module responsibilities: [PLAN.md §6](../PLAN.md)
- Entity + schema: [DATA_MODEL.md](./DATA_MODEL.md)
- Endpoint reference: [API.md](./API.md)
- Risk tier rules: [POLICY.md](./POLICY.md)
- Threat model: [THREAT_MODEL.md](./THREAT_MODEL.md)
