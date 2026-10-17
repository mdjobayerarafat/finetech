# BankPilot — Demo Credentials

All demo accounts live in the database after `bankpilot-seed --reset` runs.
**Passwords are shared / non-secret** — they exist only to drive the UI flows
(end-to-end demo, red-team, safety console). Do **not** reuse any of these
in any other environment.

---

## Quick-reference table

| Username        | Password    | Role     | Display name   | Persona                                                            |
| --------------- | ----------- | -------- | -------------- | ------------------------------------------------------------------ |
| `admin`         | `admin1234` | admin    | Demo Admin     | Full admin · audit · safety · user unlock · rule editor            |
| `auditor`       | `demo1234`  | admin    | Ops Auditor    | Read-only ops wallet · audit log only                              |
| `demo`          | `demo1234`  | customer | Alex Demo      | English · medium risk · 2 accounts · ~120 days of transactions    |
| `intl_student`  | `demo1234`  | customer | Mei Intl       | Mixed EN/中文 · low risk · student accounts                         |
| `vip_demo`      | `demo1234`  | customer | Vera VIP       | High balance · equity + bond holdings · FX-heavy outflows         |
| `locked_vip`    | `demo1234`  | customer | Lin Locked     | Pre-locked for 3 denied high-risk transfers (later admin-unlocked) |

> **Tip.** Every password except `admin` is `demo1234`. Reset the DB at any
> time with `POST /api/v1/admin/reset` (admin role required) or with
> `bankpilot-seed --reset` from the CLI.

---

## Persona deep-dive

### `admin` / `admin1234` — Demo Admin
- Role: **admin**.
- Language: English.
- Risk tolerance: medium.
- Accounts: none (admin is operator-only).
- What you can do: full `/admin/*` suite (users, rules, audit, safety, redteam).
- Use for: rule authoring, safety event review, user unlock, ledger audit.

### `auditor` / `demo1234` — Ops Auditor
- Role: **admin** (read-only ops).
- Language: English.
- Risk tolerance: low.
- Accounts: 1 (Ops wallet · `****0001` · 50 000 fen).
- What you can do: read `/admin/audit`, read `/admin/safety`, view users.
- Use for: showing the read-only side of the admin panel without spend capability.

### `demo` / `demo1234` — Alex Demo
- Role: **customer**.
- Language: English.
- Risk tolerance: medium.
- Accounts: 2 (Primary checking `****1234`, Savings `****9876`).
- Back-history: ~120 days of generated food/transport/subs/shopping +
  monthly payroll + 4 hand-rolled one-offs (Anna dinner, Li Wei coffee,
  Wang Fei repay, Apple iPad Pro).
- Holdings: wp_cash (¥2 000) + wp_bond (¥1 000).
- Safety events: `rapid_high_value` (¥9 000) and `failed_mfa` recently.
- Use for: standard "happy path" customer demo, subscription review, savings
  goal planner, fraud-flag review.

### `intl_student` / `demo1234` — Mei Intl
- Role: **customer**.
- Language: **mixed** (EN + 中文) — exercises the bilingual parser.
- Risk tolerance: **low**.
- Accounts: 2 (Student checking `****5678`, Allowance savings `****4321`).
- Back-history: ~120 days, smaller spends (scale 0.6×/0.7×).
- Holdings: wp_cash (¥500).
- Safety events: `injection_pattern` (memo trying prompt injection) and
  `denied_policy` (over nightly limit).
- Use for: bilingual chat, low-risk persona, injection-pattern redteam.

### `vip_demo` / `demo1234` — Vera VIP
- Role: **customer**.
- Language: English.
- Risk tolerance: **high**.
- Horizon: **long**.
- Accounts: 2 (VIP checking `****7777`, Treasury savings `****8888`).
- Balances: ~¥185 000 checking + ~¥1 200 000 savings (i.e. a seven-figure
  customer in fen).
- Back-history: ~150 days, larger amounts (scale 4× inflow / 3.5× spend).
- Holdings: wp_cash (¥50 000) + wp_bond (¥80 000) + wp_equity (¥120 000).
- One-offs: Marriott (Tokyo trip, ¥2 500), Hermès (gift, ¥8 900),
  Acme Corp Bonus credit (¥42 000).
- Safety events: `large_outflow` (Hermès ¥8 900).
- Use for: high-net-worth flows, equity-fund subscription, transfer-policy
  tiered approval, FX demo.

### `locked_vip` / `demo1234` — Lin Locked
- Role: **customer**.
- Language: 中文.
- Risk tolerance: medium.
- Accounts: 1 (Primary checking `****2468`).
- Initial state: `safety_lock_state = 'locked'`, `failed_mfa_count = 3`,
  reason "3 denied high-risk transfers in 5 minutes".
- Back-history: ~90 days + 3 hand-rolled transfers to a Stranger (unverified).
- The safety-events loop adds 3 × `denied_high_risk`, 1 × `admin_unlock`,
  1 × `failed_mfa`, and the final UPDATE flips the row back to
  `unlocked` with reason "verified by phone" (matching the SQL seed).
- Use for: the safety console's deny-and-unlock flow, the locked login card
  (the card on `/login` is intentionally `disabled: true`).

---

## How to (re)seed the database

### CLI
```bash
cd backendrust
DATABASE_URL="postgres://postgres:<pwd>@<host>:5432/appdb" \
  ./target/debug/bankpilot-seed --reset
```
Without `--reset` the binary only seeds an **empty** database.

### HTTP
As an admin user:
```bash
curl -X POST http://127.0.0.1:8080/api/v1/admin/reset \
  -H "X-CSRF-Token: $CSRF" -b cookies.txt
```

### What gets wiped
`reset_and_seed` DELETEs every row in this order, then re-inserts:
`audit_events · safety_events · pending_actions · workflow_nodes ·
workflows · wealth_holdings · wealth_products · cards · reservations ·
savings_goals · reminders · subscriptions · transfers · transactions ·
payees · accounts · sessions · users`.

---

## Login-page quick-fill cards

The login screen (`frontend/src/app/login/page.tsx`) advertises the same
six personas as quick-fill cards. The dictionary entries live in
`frontend/src/i18n/dictionaries.ts` under the `demo.*` keys (both `en`
and `zh`). The `locked_vip` card is rendered with `disabled: true` so
the user can't submit a locked username; the safety page is where you
unlock it.

---

## Notes

- **Argon2id hashes** are regenerated on every `seed.rs` run via
  `security::hash_password`. The SQL `seed.sql` ships pre-computed hashes
  for the SQL-only flow.
- **Audit chain.** Per the comment in `seed.sql`, the chain isn't seeded
  statically — it gets rebuilt by the running backend on the first
  state-changing request after a reset. `POST /admin/reset` itself is
  the first audit row of the new chain.
- **Demo data only.** No real bank, no real money. Every transaction
  routes through `mock://` URIs.
