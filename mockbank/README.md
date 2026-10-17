# mockbank

A minimal, real Go/Gin mock bank that the BankPilot agent and frontend can call
over HTTP. It mirrors the **Green / Yellow / Red** permission tiers from the
Rust backend, enforces a daily transfer limit, requires mock MFA on
high-value transfers, and writes every action to an append-only audit log.

> Simulation only — no real money, no real payment rails.

## Layout

```
mockbank/
├── cmd/server/main.go        # gin router, graceful shutdown
├── internal/
│   ├── config/               # env loading
│   ├── db/                   # pgx pool + schema bootstrap
│   ├── auth/                 # JWT (HS256) + dev login
│   ├── middleware/           # request id, audit, idempotency
│   ├── handlers/             # routes per resource
│   ├── policy/               # Green/Yellow/Red + daily limit
│   ├── audit/                # append-only event store
│   └── money/                # fen helpers
├── migrations/0001_init.sql  # tables, in the `mockbank` schema
├── scripts/                  # run.sh, reset.sh
└── .env.example
```

## Endpoints

| Method | Path | Tier | Notes |
|---|---|---|---|
| `GET`  | `/healthz` | — | liveness |
| `POST` | `/api/v1/auth/login` | — | dev login → JWT |
| `GET`  | `/api/v1/accounts` | Green | list user's accounts |
| `GET`  | `/api/v1/accounts/:id/balance` | Green | |
| `GET`  | `/api/v1/accounts/:id/transactions` | Green | `?from&to&limit` |
| `GET`  | `/api/v1/payees` | Green | |
| `POST` | `/api/v1/transfers` | Yellow ≤ ¥1k / Red > ¥1k | needs `X-Idempotency-Key` |
| `GET`  | `/api/v1/reports/spending` | Green | `?from&to&group_by` |
| `GET`  | `/api/v1/bills/recurring` | Green | server-detected subs |
| `GET`  | `/api/v1/subscriptions` | Green | |
| `POST` | `/api/v1/subscriptions/:id/cancel` | Yellow | |
| `POST` | `/api/v1/mfa/verify` | — | exchange one-time code for `X-MFA-Token` |
| `GET`  | `/api/v1/audit` | Green (admin) | recent events |

`X-MFA-Token` is required on any transfer that crosses the daily threshold
(¥1,000 by default, configurable via `DAILY_LIMIT_FEN`).

## Run

```bash
cd mockbank
cp .env.example .env       # set DATABASE_URL + SESSION_SECRET
./scripts/run.sh           # or: go run ./cmd/server
```

The service listens on `:8090` by default. It connects to the same Postgres
the Rust backend uses, but writes to a separate `mockbank` schema so the two
services do not collide.
