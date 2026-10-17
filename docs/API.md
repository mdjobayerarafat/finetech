# API

> REST + JSON, session cookie + CSRF. All routes under `/api/v1`. Versioned.
> Every state-changing route requires a CSRF token in `X-CSRF-Token` (or `_csrf` body field for form posts). See `SECURITY.md §5`.

## 1. Conventions

- **Content type**: `application/json; charset=utf-8`.
- **Errors**: `{ "error": { "code": "...", "message": "...", "trace_id": "..." } }`. HTTP status reflects the kind, not a generic 400.
- **Money in JSON**: integer fen. Out: `"amount_fen": 20000` for ¥200.00. We also accept a string `"amount_cny": "200.00"` on request and convert server-side; the response always uses fen.
- **Timestamps**: ISO 8601 UTC (`2026-10-16T12:34:56Z`).
- **IDs**: opaque strings (uuid v4).
- **Language**: `Accept-Language` is respected; response `Content-Language` echoes.

## 2. Auth and session

| Method | Path | Purpose | Permission |
|---|---|---|---|
| POST | `/auth/register` | Create a demo user | open |
| POST | `/auth/login` | Authenticate; sets `bp_session` cookie | open |
| POST | `/auth/logout` | Invalidate session | authenticated |
| GET | `/me` | Current user + safety lock + preferred language | authenticated |

CSRF token is fetched via `GET /auth/csrf` and returned as a cookie + JSON body. The frontend reads the cookie and echoes the value back in the `X-CSRF-Token` header for every state-changing call.

## 3. Accounts and transactions

| Method | Path | Purpose |
|---|---|---|
| GET | `/accounts` | List the authenticated user's accounts |
| GET | `/accounts/{id}/balance` | Available + reserved (masked number included) |
| GET | `/transactions?account_id=…&from=…&to=…&category=…&limit=…` | Filtered ledger |
| GET | `/reports/spending?from=…&to=…&group_by=category\|day` | Backend-calculated totals |
| GET | `/reports/anomalies?from=…&to=…` | Rule-based anomaly candidates |
| GET | `/reports/monthly?year=YYYY&month=MM` | Income, expenses, category totals, MoM comparison |
| GET | `/reports/annual?year=YYYY` | Annual aggregation with missing-coverage notice |

Reports are **deterministic**: the API computes totals from the ledger; the LLM only narrates.

## 4. Payees and transfers

| Method | Path | Purpose |
|---|---|---|
| GET | `/payees` | List mock contacts |
| POST | `/payees` | Add a new payee (verified flag starts false) |
| GET | `/transfers?status=…&limit=…` | Transfer history |
| POST | `/transfers/preview` | Dry-run: returns recipient resolution, masked destination, daily-remaining, tier, expiry |
| POST | `/transfers` | Create a transfer request; returns `pending_action_id` if Yellow/Red |
| POST | `/transfers/{id}/cancel` | Cancel a pending or scheduled transfer |

## 5. Subscriptions and reminders

| Method | Path | Purpose |
|---|---|---|
| GET | `/subscriptions` | Detected recurring charges + evidence |
| POST | `/subscriptions/{id}/remind` | Create a renewal reminder |
| POST | `/subscriptions/{id}/cancel` | Initiate sim-cancellation via mock merchant connector |
| GET | `/reminders` | List reminders |
| DELETE | `/reminders/{id}` | Cancel a reminder |

## 6. Cards (stretch, scenario 6)

| Method | Path | Purpose | Tier |
|---|---|---|---|
| GET | `/cards` | List virtual cards | Green |
| POST | `/cards` | Issue a new virtual card | Yellow |
| POST | `/cards/{id}/freeze` | Freeze (temporary block) | Yellow |
| POST | `/cards/{id}/unfreeze` | Unfreeze | Yellow |
| POST | `/cards/{id}/report-lost` | Mark lost; immediate cancel + reissue | Red |
| POST | `/cards/{id}/cancel` | Cancel permanently | Yellow |
| PATCH | `/cards/{id}/limits` | Update single-tx and/or monthly limit | Yellow |

## 7. Wealth (stretch, scenario 5)

| Method | Path | Purpose | Tier |
|---|---|---|---|
| GET | `/wealth/products` | Catalog (read-only) | Green |
| GET | `/wealth/products/{id}` | Detail with suitability | Green |
| GET | `/wealth/holdings` | User's holdings | Green |
| POST | `/wealth/subscribe` | Subscribe (sim fulfilment) | Yellow if suitability passed; Red otherwise |
| POST | `/wealth/redeem` | Redeem (sim fulfilment) | Yellow |

Suitability rules are read by the policy engine; the LLM never decides whether a product is suitable. See `POLICY.md §5`.

## 8. Agent, planner, workflows

| Method | Path | Purpose |
|---|---|---|
| POST | `/agent/messages` | Submit a user message → returns intent + plan + (optional) `clarification_required` + (optional) `pending_action_id`s |
| GET | `/workflows/{id}` | Plan + node states + DAG |
| POST | `/workflows/{id}/pause` | Stop scheduling; in-flight node completes atomically |
| POST | `/workflows/{id}/cancel` | Cancel pending nodes; compensate where safe |
| POST | `/workflows/{id}/resume` | Resume from paused (after re-confirming nodes) |

## 9. Approvals and MFA

| Method | Path | Purpose |
|---|---|---|
| GET | `/actions/{id}` | Inspect a pending action (parameters, tier, expiry) |
| POST | `/actions/{id}/confirm` | Yellow confirmation; checks parameter hash |
| POST | `/actions/{id}/verify` | Red verification: submit mock MFA challenge response |
| POST | `/actions/{id}/execute` | Run an approved, verified action (also called implicitly by workflow executor) |

Parameter hash is the **canonicalised** (sorted keys, lowercased) SHA-256 of the parameters. Any client-side change between `/confirm` and `/execute` is rejected.

## 10. Audit and safety (admin)

| Method | Path | Purpose |
|---|---|---|
| GET | `/audit?from=…&to=…&action=…` | Filtered audit events (user-scoped for customers; full for admin) |
| GET | `/audit/verify-chain?from=…&to=…` | Admin: walk the hash chain and report breaks |
| GET | `/admin/safety-events` | Admin: list safety events |
| POST | `/admin/safety-lock/{user_id}/unlock` | Admin unlock (logged) |
| POST | `/admin/reset` | Admin: wipe + reseed demo data |

## 11. Error codes

| HTTP | `code` | When |
|---|---|---|
| 400 | `schema_invalid` | JSON / parameter schema violation |
| 400 | `clarification_required` | Agent needs more info; `details.questions[]` included |
| 401 | `unauthenticated` | No / expired session |
| 403 | `forbidden` | Role gate denied |
| 403 | `csrf_invalid` | CSRF token missing or wrong |
| 403 | `policy_denied` | Tier would be downgraded; not allowed |
| 403 | `safety_locked` | User is in lock state |
| 404 | `not_found` | Resource doesn't exist or isn't owned |
| 409 | `parameter_changed` | Parameter hash mismatch on confirm/verify/execute |
| 409 | `idempotency_replay` | Same key, different parameters |
| 410 | `expired` | Pending action past expiry |
| 422 | `daily_limit_exceeded` | Cannot bypass via this channel |
| 422 | `suitability_failed` | Wealth product not suitable |
| 500 | `internal_error` | Server bug; logged with trace_id |

## 12. Example: smart transfer (mixed-language)

Request:

```http
POST /api/v1/agent/messages
Cookie: bp_session=...
X-CSRF-Token: ...
Content-Type: application/json

{ "text": "转 200 给 Li Wei，午饭钱" }
```

Response:

```json
{
  "intent": "transfer",
  "language": "mixed",
  "entities": { "recipient_query": "Li Wei", "amount_fen": 20000, "memo": "午饭钱" },
  "clarification_required": false,
  "plan": {
    "workflow_id": "wf_01HXY…",
    "nodes": [
      {
        "index": 0,
        "tool": "transfer.create_pending",
        "risk_tier": "Yellow",
        "approval_required": true,
        "parameters": {
          "source_account_id": "acc_01H…",
          "payee_id": "payee_01H…",
          "amount_fen": 20000,
          "memo": "午饭钱"
        }
      }
    ]
  },
  "pending_action": {
    "id": "act_01H…",
    "expires_at": "2026-10-16T13:00:00Z",
    "parameter_hash": "9f3a…"
  }
}
```

The frontend then renders the confirmation card. On click:

```http
POST /api/v1/actions/act_01H…/confirm
X-CSRF-Token: ...
```

…which moves the pending action to `confirmed`, then the workflow executor runs the node, posts the ledger entry, and writes the audit event.
