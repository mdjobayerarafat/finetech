# BankPilot — Rust Backend (Actix Web)

This is a Rust port of `backend/` (FastAPI). The Python implementation is kept
intact; the new Rust binary lives here, side by side.

## What was ported

- **All routes** under `/api/v1` (auth, accounts, payees, transfers,
  subscriptions, cards, wealth, agent, workflows, actions, audit, admin).
- **Schema** as a `sqlx` migration under `migrations/`.
- **Policy engine** (Green / Yellow / Red) and tier escalation.
- **Audit log** with SHA-256 hash chain + chain-verify endpoint.
- **Safety lock** (failed MFA, denied high-risk, rapid high-value, injection).
- **Argon2id** password hashing.
- **Session + CSRF** cookies (HMAC-signed session id, plain CSRF).
- **Tool registry** + **DAG planner** + deterministic LLM stub.
- **Seed** script that recreates the Python demo data.

## Layout

```
backendrust/
├── Cargo.toml
├── migrations/         # sqlx-managed SQLite schema
├── run.sh              # cargo run --release --bin bankpilot
└── src/
    ├── main.rs
    ├── lib.rs
    ├── config.rs       # env-driven settings
    ├── db.rs           # sqlx pool + migrator
    ├── errors.rs       # ApiError envelope
    ├── ids.rs / money.rs / canonical.rs
    ├── models.rs       # sqlx FromRow row types
    ├── sessions.rs / security.rs / state.rs / ctx.rs
    ├── audit.rs / policy.rs / safety.rs
    ├── tools.rs        # tool registry + executor
    ├── planner.rs      # workflow + DAG
    ├── llm.rs          # deterministic stub (online path stubbed)
    ├── seed.rs
    └── routes/         # actix-web handlers
```

## Running

```bash
cd backendrust
cargo run --release --bin bankpilot
```

Defaults to port `8080` (override with `PORT=...`). It reads the same `.env`
file at the repo root as the Python backend.

To seed (or reset) the demo data:

```bash
cargo run --release --bin bankpilot-seed         # create if empty
cargo run --release --bin bankpilot-seed -- --reset   # wipe + reseed
```

## Differences from the Python backend

- The LLM HTTP client is **not** wired (the Python backend falls back to the
  stub on any failure, so behaviour is identical by default).
- The OpenAPI schema is not auto-served; the `/api/v1/docs` route returns a
  short pointer to the Python backend's docs.
- Static SPA mount from `frontend/out` is omitted — the Rust backend
  serves only JSON.
- The SQLite driver handles dates slightly differently; the audit hash
  canonicalisation normalises both sides to UTC.

## Smoke test

```bash
curl -s http://localhost:8080/healthz
curl -s -X POST http://localhost:8080/api/v1/auth/login \
    -H 'Content-Type: application/json' \
    -d '{"username":"demo","password":"demo1234"}'
```
