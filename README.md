# BankPilot

> Secure, multilingual AI banking agent — simulation only, no real money.

BankPilot helps users express everyday financial intent in **English, Chinese, or mixed input** and turns it into a structured, server-checked plan. The language model proposes; a deterministic policy engine authorises; the backend executes; an append-only audit log records everything.

> ⚠️ **Simulation only — no real bank accounts, no real payments.** Every screen and demo surface says so.

## What's in here

| | |
|---|---|
| [PLAN.md](./PLAN.md) | Master plan and phase index |
| [docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md) | Component map, request lifecycle, deployment |
| [docs/DATA_MODEL.md](./docs/DATA_MODEL.md) | Entities, schema, money handling |
| [docs/API.md](./docs/API.md) | REST endpoints, request/response schemas |
| [docs/POLICY.md](./docs/POLICY.md) | Green / Yellow / Red engine, escalation |
| [docs/SECURITY.md](./docs/SECURITY.md) | Threat model, defenses, security test matrix |
| [docs/self-assessment/](./docs/self-assessment/README.md) | External security self-assessment, bilingual (Phase 7 deliverable) |
| [docs/SCENARIOS.md](./docs/SCENARIOS.md) | The 6 user scenarios, demo prompts |
| [docs/AGENT_PIPELINE.md](./docs/AGENT_PIPELINE.md) | Intent → slots → DAG → execution |
| [docs/DEMO_SCRIPT.md](./docs/DEMO_SCRIPT.md) | 5-minute walkthrough for judges |
| [docs/TESTING.md](./docs/TESTING.md) | Functional + security test plan |
| [docs/ROADMAP.md](./docs/ROADMAP.md) | 7-phase build order, exit criteria |
| [docs/THREAT_MODEL.md](./docs/THREAT_MODEL.md) | STRIDE-style analysis, mitigations |

## Quickstart (once Build is running)

```bash
# 1. env
cp .env.example .env
# Fill DATABASE_URL (Coolify Postgres), OPENAI_* (or leave blank for stub)

# 2. database
cd backend && sqlx database create && sqlx migrate run && cargo run --bin seed

# 3. run
cargo run --bin bankpilot-api    # backend on :8080
cd ../frontend && pnpm dev       # frontend on :3000
```

Demo accounts (seeded):

| Role | Username | Password |
|---|---|---|
| Customer | `demo` | `demo1234` |
| Customer | `intl_student` | `demo1234` |
| Admin | `admin` | `admin1234` |

## Scenarios shipped

1. Smart transfers (name, phone tail, scheduled, split-bill)
2. Bill analysis (category, monthly, annual, anomalies)
3. Subscriptions & direct debit (detect, remind, simulated cancel)
4. Cross-scenario orchestration (birthday / rent DAG with pause + compensation)
5. Wealth operations (catalog read + suitability + simulated fulfilment) — stretch
6. Card management (virtual card lifecycle in simulation) — stretch

## Non-negotiables

- Money is **integer minor units** (fen). No `f64` in ledger math.
- The LLM **never** chooses its own risk tier; the policy engine does.
- **Yellow** = one confirmation · **Red** = confirmation + mock MFA.
- Untrusted text (memos, payee names, merchant fields) **cannot** issue instructions.
- Audit log is **append-only** with hash chain. No secrets in logs.
- Idempotency keys on every state-changing call.

## Status

- **Phase 0 — Plan**: ✅ (this commit)
- **Phase 1 — Foundation**: pending Build
- See [docs/ROADMAP.md](./docs/ROADMAP.md) for the full schedule.
