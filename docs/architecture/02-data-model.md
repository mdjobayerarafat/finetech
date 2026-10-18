---
id: arch-data-model
title: "2 · Data Model"
sidebar_label: "2 · Data Model"
sidebar_position: 3
---

# 2. Data Model

> 中文: [02-数据模型 (中文)](./zh/02-data-model.md)

The schema is plain PostgreSQL, expressed in 6 migration files under
`backendrust/migrations/`. Migrations are **idempotent** and
applied in filename order.

## 2.1 Entity-relationship diagram

```mermaid
erDiagram
    USERS ||--o{ SESSIONS : "has"
    USERS ||--o{ ACCOUNTS : "owns"
    USERS ||--o{ PAYEES : "owns"
    USERS ||--o{ CARDS : "has"
    USERS ||--o{ SAVINGS_GOALS : "tracks"
    USERS ||--o{ SUBSCRIPTIONS : "tracks"
    USERS ||--o{ REMINDERS : "tracks"
    USERS ||--o{ WEALTH_HOLDINGS : "owns"
    USERS ||--o{ BUDGETS : "owns"
    USERS ||--o{ BUDGET_RESERVATIONS : "owns"
    USERS ||--o{ CONVERSATIONS : "has"
    USERS ||--o{ PENDING_ACTIONS : "awaits"
    USERS ||--o{ SAFETY_EVENTS : "produced"
    USERS ||--o{ AGENT_RULES : "created_by (admin)"
    USERS ||--o{ AUDIT_EVENTS : "actor"
    USERS ||--o{ WORKFLOWS : "owns"

    ACCOUNTS ||--o{ TRANSACTIONS : "posts"
    ACCOUNTS ||--o{ TRANSFERS : "source of"
    ACCOUNTS ||--o{ RESERVATIONS : "source of"
    ACCOUNTS ||--o{ BUDGET_RESERVATIONS : "source of"
    ACCOUNTS ||--o{ SPLIT_BILLS : "source of"

    PAYEES ||--o{ TRANSFERS : "destination of"
    PAYEES ||--o{ SPLIT_BILL_PARTS : "participant"

    TRANSFERS ||--o{ SPLIT_BILL_PARTS : "paid via"

    SAVINGS_GOALS ||--o{ RESERVATIONS : "held against"
    SAVINGS_GOALS ||--o{ BUDGET_RESERVATIONS : "held against"

    SUBSCRIPTIONS ||--o{ REMINDERS : "linked"

    WEALTH_PRODUCTS ||--o{ WEALTH_HOLDINGS : "subscribed"

    BUDGETS ||--o{ BUDGET_RESERVATIONS : "anchors"

    CONVERSATIONS ||--o{ CONVERSATION_MESSAGES : "contains"
    USERS ||--o{ CONVERSATION_MESSAGES : "author of"

    WORKFLOWS ||--o{ WORKFLOW_NODES : "executes"
    WORKFLOW_NODES ||--o{ PENDING_ACTIONS : "may await"

    SPLIT_BILLS ||--o{ SPLIT_BILL_PARTS : "broken into"

    USERS {
      TEXT id PK
      TEXT username UK
      TEXT password_hash
      TEXT display_name
      TEXT preferred_language
      TEXT role
      TEXT safety_lock_state
      TEXT safety_lock_reason
      INT  failed_mfa_count
      TEXT risk_tolerance
      TEXT horizon
    }
    SESSIONS {
      TEXT id PK
      TEXT user_id FK
      TEXT csrf_token
      TIMESTAMPTZ expires_at
    }
    ACCOUNTS {
      TEXT id PK
      TEXT user_id FK
      TEXT display_name
      TEXT masked_number
      TEXT currency
      BIGINT available_balance_fen
      BIGINT reserved_balance_fen
    }
    TRANSACTIONS {
      TEXT id PK
      TEXT account_id FK
      BIGINT amount_fen
      TEXT direction
      TEXT category
      TEXT merchant
      TEXT memo
      TIMESTAMPTZ occurred_at
      TEXT status
    }
    PAYEES {
      TEXT id PK
      TEXT owner_user_id FK
      TEXT display_name
      TEXT phone_tail
      TEXT mock_destination
      BOOLEAN verified
      TIMESTAMPTZ first_used_at
    }
    TRANSFERS {
      TEXT id PK
      TEXT source_account_id FK
      TEXT payee_id FK
      BIGINT amount_fen
      TEXT memo
      TIMESTAMPTZ schedule
      TEXT idempotency_key UK
      TEXT status
      TIMESTAMPTZ posted_at
    }
    SUBSCRIPTIONS {
      TEXT id PK
      TEXT user_id FK
      TEXT merchant
      BIGINT estimated_amount_fen
      INT  interval_days
      TIMESTAMPTZ next_expected_at
      TIMESTAMPTZ last_charge_at
      JSONB evidence
      TEXT cancellation_status
    }
    REMINDERS {
      TEXT id PK
      TEXT user_id FK
      TEXT kind
      TEXT subject
      TIMESTAMPTZ due_at
      TEXT status
      TEXT linked_subscription_id FK
    }
    SAVINGS_GOALS {
      TEXT id PK
      TEXT user_id FK
      TEXT name
      BIGINT target_fen
    }
    RESERVATIONS {
      TEXT id PK
      TEXT goal_id FK
      BIGINT amount_fen
      TEXT state
      TEXT source_account_id FK
    }
    CARDS {
      TEXT id PK
      TEXT user_id FK
      TEXT display_name
      TEXT masked_number
      TEXT status
      BIGINT single_tx_limit_fen
      BIGINT monthly_limit_fen
    }
    WEALTH_PRODUCTS {
      TEXT id PK
      TEXT name
      TEXT category
      TEXT risk_level
      DOUBLE PRECISION expected_annual_return_pct
      BIGINT min_subscription_fen
      TEXT suitability_required
    }
    WEALTH_HOLDINGS {
      TEXT id PK
      TEXT user_id FK
      TEXT product_id FK
      BIGINT principal_fen
      TIMESTAMPTZ started_at
    }
    BUDGETS {
      TEXT id PK
      TEXT user_id FK
      TEXT category
      BIGINT limit_fen
      TIMESTAMPTZ period_start
      TIMESTAMPTZ period_end
      TEXT account_id FK
      TEXT note
    }
    BUDGET_RESERVATIONS {
      TEXT id PK
      TEXT user_id FK
      TEXT budget_id FK
      TEXT goal_id FK
      TEXT label
      BIGINT amount_fen
      TIMESTAMPTZ due_at
      TEXT source_account_id FK
      TEXT state
    }
    PENDING_ACTIONS {
      TEXT id PK
      TEXT user_id FK
      TEXT workflow_id FK
      TEXT node_id FK
      TEXT tool
      JSONB arguments
      TEXT risk_tier
      TEXT policy_reason
      TEXT parameter_hash
      TIMESTAMPTZ expires_at
      TEXT status
    }
    SAFETY_EVENTS {
      TEXT id PK
      TEXT user_id FK
      TEXT kind
      TEXT details
      TIMESTAMPTZ created_at
    }
    AGENT_RULES {
      TEXT id PK
      TEXT kind
      TEXT name
      TEXT pattern
      TEXT severity
      TEXT applies_to
      BOOLEAN enabled
      TEXT created_by FK
    }
    AUDIT_EVENTS {
      TEXT id PK
      TIMESTAMPTZ created_at
      TEXT actor_user_id FK
      TEXT session_id
      TEXT request_id
      TEXT workflow_id
      TEXT node_id
      TEXT action
      TEXT tool
      JSONB parameters_masked
      TEXT risk_tier
      TEXT policy_reason
      TEXT confirmation_status
      TEXT mfa_status
      TEXT result
      TEXT error_code
      TEXT prev_hash
      TEXT hash
    }
    WORKFLOWS {
      TEXT id PK
      TEXT user_id FK
      TEXT status
      JSONB plan
    }
    WORKFLOW_NODES {
      TEXT id PK
      TEXT workflow_id FK
      TEXT tool
      JSONB arguments
      TEXT risk_tier
      TEXT status
      TEXT idempotency_key
    }
    CONVERSATIONS {
      TEXT id PK
      TEXT user_id FK
      TEXT title
    }
    CONVERSATION_MESSAGES {
      TEXT id PK
      TEXT conversation_id FK
      TEXT user_id FK
      TEXT role
      TEXT content
      JSONB meta
    }
    SPLIT_BILLS {
      TEXT id PK
      TEXT owner_user_id FK
      TEXT title
      TEXT memo
      BIGINT total_fen
      TEXT currency
      TEXT source_account_id FK
      TEXT status
      TIMESTAMPTZ due_at
    }
    SPLIT_BILL_PARTS {
      TEXT id PK
      TEXT split_bill_id FK
      TEXT payee_id FK
      TEXT participant_name
      TEXT participant_phone_tail
      BIGINT share_fen
      TEXT status
      TIMESTAMPTZ paid_at
      TEXT paid_transfer_id FK
    }
```

## 2.2 Migration timeline

| File | Introduces |
|---|---|
| `20251020000000_init_pg.sql` | `users`, `sessions`, `accounts`, `transactions`, `payees`, `transfers`, `subscriptions`, `reminders`, `savings_goals`, `reservations`, `cards`, `wealth_products`, `wealth_holdings` |
| `20251115000000_agent_v2.sql` | `workflows`, `workflow_nodes`, `pending_actions`, `safety_events`, `audit_events` (with hash chain) |
| `20251118000000_chat_memory.sql` | `conversations`, `conversation_messages` |
| `20251120000000_smart_transfers.sql` | `split_bills`, `split_bill_parts`, status recompute trigger |
| `20251121000000_admin_panel.sql` | `agent_rules`, admin monitoring indexes on `audit_events`, `safety_events`, `conversations`, `workflows` |
| `20251122000000_budgets.sql` | `budgets`, `budget_reservations` |

## 2.3 Money is always integer fen

**Every** money column is `BIGINT *_fen` (1/100 CNY). There is **no
`f64` for ledger math anywhere**. This is enforced by both the
schema (`CHECK (amount_fen > 0)` on `transfers`, `budgets`,
`budget_reservations`, `split_bills`, `split_bill_parts`) and by
the rule "the planner never produces a non-integer value for
`amount_fen`."

| Field | Type | Where it is set | What coerces to fen |
|---|---|---|---|
| `accounts.available_balance_fen` | BIGINT | `t_transfer_create_pending`, `t_card_*`, seed | Always integer |
| `accounts.reserved_balance_fen` | BIGINT | `t_goal_reservation` (Yellow) → `t_budget_reserve` (Red) | Always integer |
| `transfers.amount_fen` | BIGINT | planner | `money::coerce_amount_fen` from CNY string |
| `transactions.amount_fen` | BIGINT | tool side effect | sign = `direction` |
| `subscriptions.estimated_amount_fen` | BIGINT | `detect_subscriptions` heuristic | LLM-stripped or rule-stripped |
| `budgets.limit_fen` | BIGINT | `t_budget_set` | Yellow tier, integer only |
| `budget_reservations.amount_fen` | BIGINT | `t_budget_reserve` | Red tier, integer only |
| `wealth_holdings.principal_fen` | BIGINT | `t_wealth_subscribe` | Yellow tier after suitability check |
| `split_bills.total_fen` / `share_fen` | BIGINT | `smart_transfers::create_split_bill` | sum invariant checked in app code |

> **Display rule.** Frontend formats with `Intl.NumberFormat` using
> the locale's currency style. There is **no** server-side float
> conversion; the only conversion to decimal happens in the UI.

## 2.4 The audit hash chain

`audit_events` is **append-only** with a SHA-256 chain. Every row
records `prev_hash` and `hash` where:

```
hash = sha256(
    canonical_json({
        actor_user_id, session_id, request_id,
        workflow_id, node_id, action, tool,
        parameters_masked, risk_tier, policy_reason,
        confirmation_status, mfa_status, result, error_code,
        created_at, prev_hash,
    })
)
```

The genesis hash is `00…00` (32 bytes). `audit::last_hash` always
reads the most recent row. `audit::verify_chain` walks the table
and recomputes the digest, in O(n). Any tampering — including
modifying `parameters_masked` after the fact — breaks the chain
on the next verify.

`parameters_masked` is the field that prevents the chain from
leaking secrets. Sensitive values are replaced with a placeholder
like `***` before the canonical JSON is hashed; the raw value is
**never** written to the audit log.

## 2.5 Indexes

Critical query paths are indexed:

- `ix_transactions_account_occurred (account_id, occurred_at)` —
  the dashboard's 12-month chart query.
- `ix_audit_events_user_time (actor_user_id, created_at DESC)` —
  the admin audit page.
- `ix_safety_events_user_time (user_id, created_at DESC)` — the
  admin safety page.
- `ix_conversation_messages_user_time (user_id, created_at DESC)` —
  the chat history list.
- `ix_workflows_user_time (user_id, created_at DESC)` — the
  workflows list.
- `ix_agent_rules_enabled (enabled, kind)` — the rules engine
  hot path.
- `idx_budgets_user_category (user_id, category)` — the budget
  cap lookup.
- `ix_split_bills_status (status)` — the "what's still owed"
  filter.

## 2.6 Concurrency model

- All state-changing writes happen in a single Postgres
  transaction.
- The `accounts` row is taken with `SELECT … FOR UPDATE` before
  the balance is decremented. This is the **only** way to write
  `available_balance_fen`, so two parallel transfer requests for
  the same account serialise instead of double-spending.
- The daily transfer cap (`policy::check_daily_limit`) reads and
  sums `transfers` in the same transaction that inserts the new
  row, so a parallel approval cannot exceed the cap.
- Workflow nodes use `workflow_id + node_id + parameter_hash` as
  the idempotency key. A retry is a no-op.
