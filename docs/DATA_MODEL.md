# Data Model

> Money is **integer minor units** (fen for CNY). This is a hard rule — see `PLAN.md §5` rule 1. Any code path that converts a money value with `f64` fails review.

## 1. Conventions

- **IDs**: `uuid v4` text columns (`text` type, no `uuid` Postgres type to keep `sqlx` offline checks simple).
- **Timestamps**: `timestamptz` (UTC). The API converts to the user's preferred locale at the edge.
- **Money**: `BIGINT` columns named `*_fen`. Display layer formats to CNY with `¥` and 2 decimals.
- **Enums**: stored as `text` with a CHECK constraint and a Rust enum on the read side.
- **Soft fields**: nullable rather than sentinel values (e.g. `cancelled_at` rather than `status = 'cancelled'` with a separate timestamp).
- **Seed data only** — no real PII. See `THREAT_MODEL.md §6`.

## 2. Entities

### 2.1 `users`

| Column | Type | Notes |
|---|---|---|
| `id` | text PK | |
| `username` | text unique | |
| `password_hash` | text | Argon2id |
| `display_name` | text | |
| `preferred_language` | text CHECK in (`en`,`zh`,`mixed`) | |
| `role` | text CHECK in (`customer`,`admin`) | |
| `safety_lock_state` | text CHECK in (`unlocked`,`locked`) | default `unlocked` |
| `safety_lock_reason` | text nullable | |
| `failed_mfa_count` | int | resets on success |
| `created_at` | timestamptz | |

### 2.2 `accounts`

| Column | Type | Notes |
|---|---|---|
| `id` | text PK | |
| `user_id` | text FK → users | |
| `display_name` | text | e.g. "Primary checking" |
| `masked_number` | text | e.g. `****1234` |
| `currency` | text | default `CNY` |
| `available_balance_fen` | bigint | |
| `reserved_balance_fen` | bigint | sums reservations; not spendable |
| `created_at` | timestamptz | |

### 2.3 `transactions`

| Column | Type | Notes |
|---|---|---|
| `id` | text PK | |
| `account_id` | text FK | |
| `amount_fen` | bigint | positive for credit, negative for debit |
| `direction` | text CHECK in (`credit`,`debit`) | |
| `category` | text | e.g. `food`,`transport`,`subscription`,`transfer`,`income` |
| `merchant` | text | untrusted; sandboxed in prompts |
| `memo` | text | untrusted |
| `occurred_at` | timestamptz | |
| `status` | text CHECK in (`posted`,`pending`,`reversed`) | |

### 2.4 `payees`

| Column | Type | Notes |
|---|---|---|
| `id` | text PK | |
| `owner_user_id` | text FK | |
| `display_name` | text | |
| `phone_tail` | text nullable | last 4 digits; matched for "send to contact ending 1234" |
| `mock_destination` | text | masked; not a real account |
| `verified` | bool | determines if unverified-payee escalation fires |
| `first_used_at` | timestamptz nullable | for new-payee escalation |

### 2.5 `transfers`

| Column | Type | Notes |
|---|---|---|
| `id` | text PK | |
| `source_account_id` | text FK | |
| `payee_id` | text FK | |
| `amount_fen` | bigint | > 0 |
| `memo` | text | untrusted |
| `schedule` | text nullable | ISO 8601 datetime, or `null` for immediate |
| `idempotency_key` | text unique | |
| `status` | text CHECK in (`pending_approval`,`approved`,`scheduled`,`posted`,`failed`,`reversed`) | |
| `created_at` / `posted_at` | timestamptz | |

### 2.6 `subscriptions`

| Column | Type | Notes |
|---|---|---|
| `id` | text PK | |
| `user_id` | text FK | |
| `merchant` | text | |
| `estimated_amount_fen` | bigint | |
| `interval_days` | int | e.g. 30 |
| `next_expected_at` | timestamptz | |
| `last_charge_at` | timestamptz nullable | |
| `evidence` | jsonb | list of transaction IDs and rule hits that flagged this |
| `cancellation_status` | text CHECK in (`active`,`cancellation_requested`,`cancelled`,`failed`) | |
| `cancelled_at` | timestamptz nullable | |

### 2.7 `reminders`

| Column | Type | Notes |
|---|---|---|
| `id` | text PK | |
| `user_id` | text FK | |
| `kind` | text | `renewal`,`custom` |
| `subject` | text | |
| `due_at` | timestamptz | |
| `status` | text CHECK in (`scheduled`,`fired`,`cancelled`) | |
| `linked_subscription_id` | text nullable | |

### 2.8 `savings_goals` and `reservations`

`savings_goals`

| Column | Type |
|---|---|
| `id` | text PK |
| `user_id` | text FK |
| `name` | text |
| `target_fen` | bigint |
| `created_at` | timestamptz |

`reservations` (money earmarked but not moved)

| Column | Type |
|---|---|
| `id` | text PK |
| `goal_id` | text FK |
| `amount_fen` | bigint |
| `state` | text CHECK in (`held`,`released`,`consumed`) |
| `source_account_id` | text FK |
| `created_at` | timestamptz |

### 2.9 `cards` (stretch, scenario 6)

| Column | Type | Notes |
|---|---|---|
| `id` | text PK | |
| `user_id` | text FK | |
| `display_name` | text | |
| `masked_number` | text | `****0000` |
| `expiry_month` / `expiry_year` | int | |
| `status` | text CHECK in (`issued`,`active`,`frozen`,`lost`,`cancelled`,`expired`) | |
| `single_tx_limit_fen` | bigint nullable | |
| `monthly_limit_fen` | bigint nullable | |
| `created_at` | timestamptz | |

### 2.10 `wealth_products` and `wealth_holdings` (stretch, scenario 5)

`wealth_products` (catalog, read-only seed)

| Column | Type |
|---|---|
| `id` | text PK |
| `name` | text |
| `category` | text (`money_market`,`bond`,`fund`,`simulation_only`) |
| `risk_level` | text CHECK in (`low`,`medium`,`high`) |
| `expected_annual_return_pct` | numeric(5,2) |
| `min_subscription_fen` | bigint |
| `suitability_required` | text | e.g. `risk_tolerance:medium` |

`wealth_holdings`

| Column | Type |
|---|---|
| `id` | text PK |
| `user_id` | text FK |
| `product_id` | text FK |
| `amount_fen` | bigint |
| `subscribed_at` | timestamptz |
| `status` | text CHECK in (`active`,`redeemed`) |

### 2.11 `workflows` and `workflow_nodes`

`workflows`

| Column | Type |
|---|---|
| `id` | text PK |
| `user_id` | text FK |
| `request_id` | text |
| `plan_version` | int |
| `status` | text CHECK in (`draft`,`awaiting_confirmation`,`ready`,`running`,`paused`,`completed`,`failed`,`cancelled`,`compensation_required`) |
| `interrupt_reason` | text nullable |
| `created_at` | timestamptz |

`workflow_nodes`

| Column | Type |
|---|---|
| `id` | text PK |
| `workflow_id` | text FK |
| `node_index` | int |
| `tool` | text |
| `parameters` | jsonb (validated) |
| `dependencies` | int[] | node indexes this waits for |
| `risk_tier` | text |
| `approval_required` | bool |
| `idempotency_key` | text unique |
| `status` | text CHECK in (`pending`,`awaiting_confirmation`,`awaiting_mfa`,`running`,`completed`,`failed`,`paused`,`compensated`,`skipped`) |
| `result` | jsonb nullable |
| `error` | jsonb nullable |
| `compensating_node_id` | text nullable |

### 2.12 `pending_actions`

| Column | Type |
|---|---|
| `id` | text PK |
| `user_id` | text FK |
| `tool` | text |
| `parameters` | jsonb |
| `parameter_hash` | text | sha256 of canonicalised parameters |
| `risk_tier` | text |
| `expires_at` | timestamptz |
| `confirmation_status` | text CHECK in (`none`,`confirmed`,`expired`,`invalidated`) |
| `mfa_status` | text CHECK in (`none`,`verified`,`failed`,`expired`) |
| `created_at` | timestamptz |

### 2.13 `audit_events` (append-only)

| Column | Type |
|---|---|
| `id` | text PK |
| `created_at` | timestamptz |
| `actor_user_id` | text nullable |
| `session_id` | text nullable |
| `request_id` | text nullable |
| `workflow_id` | text nullable |
| `node_id` | text nullable |
| `action` | text |
| `tool` | text nullable |
| `parameters_masked` | jsonb |
| `risk_tier` | text nullable |
| `policy_reason` | text nullable |
| `confirmation_status` | text nullable |
| `mfa_status` | text nullable |
| `result` | text CHECK in (`ok`,`error`,`denied`,`blocked`) |
| `error_code` | text nullable |
| `prev_hash` | text |
| `hash` | text |

> **Never** stored: passwords, OTPs, API keys, full account numbers, plain-text secrets, or unnecessary PII. See `SECURITY.md §6`.

### 2.14 `safety_events`

| Column | Type |
|---|---|
| `id` | text PK |
| `user_id` | text FK |
| `kind` | text (`failed_mfa`,`denied_high_risk`,`rapid_high_value`,`injection_pattern`,`admin_unlock`) |
| `created_at` | timestamptz |
| `metadata` | jsonb |

## 3. Money math

```rust
// BAD — never do this for ledger math
let total = 0.1_f64 + 0.2_f64; // 0.30000000000000004

// GOOD — integer fen
let a: i64 = 10_000;   // ¥100.00
let b: i64 = 20_000;   // ¥200.00
let sum = a + b;       // 30_000 fen = ¥300.00, exact
```

Display formatting is the only place `f64` (or a decimal crate) is allowed, and it must be a one-way conversion from fen.

## 4. Ownership and access scoping

Every query that touches user-owned data has a `WHERE user_id = $session_user_id` clause. This is enforced in two ways:

1. **Code review**: a test suite (`TESTING.md §3`) explicitly tries to read across users and asserts the API returns 404.
2. **Session-only identity**: there is no client-supplied `user_id` in any request body. The auth middleware injects the identity from the cookie.

## 5. Migrations and seed

- All schema changes are SQLx migrations in `db/migrations/`. They run on `sqlx migrate run`.
- The seed binary (`cargo run --bin seed`) wipes and re-seeds: two demo customers, one admin, two accounts each, ~120 days of synthetic transactions, a few payees, a few recurring subscriptions, a couple of wealth products, and a sample virtual card.
- The reset script (`ops/reset.sh`) is the demo-day panic button.
