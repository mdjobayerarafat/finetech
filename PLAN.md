# BankPilot — Master Plan

> **Status:** Plan, awaiting Build.
> **Spec source:** `BankPilot_Project_Details_and_Feature_Specification.docx` (v1.0, 16 Oct 2026).
> **Tagline:** "AI may propose and explain, but it must not grant itself financial authority."

---

## 1. Project Identity

| | |
|---|---|
| **Name** | BankPilot (智银助手) |
| **Type** | AI agent / fintech prototype / competition submission |
| **Mode** | Simulation only — no real bank accounts, no real payment rails |
| **Stack** | Rust + Actix Web · PostgreSQL (Coolify) · Next.js 14 + TypeScript + Tailwind · OpenAI-compatible LLM |
| **Auth** | Session cookie + CSRF (server-rendered feel) |
| **DB** | External PostgreSQL via `DATABASE_URL` from `.env` (no docker-postgres) |
| **LLM** | OpenAI-compatible API via env (`OPENAI_BASE_URL`, `OPENAI_API_KEY`, `OPENAI_MODEL`); deterministic stub when key missing |

## 2. Scope — All 6 Scenarios

| # | Scenario | Depth | Status |
|---|---|---|---|
| 1 | Smart transfers | Full | Core |
| 2 | Bill analysis | Full | Core |
| 3 | Subscriptions / direct debit | Full | Core |
| 4 | Cross-scenario orchestration (DAG) | Full | Showcase |
| 5 | Wealth operations | Read catalog + suitability rules + simulated fulfilment | Stretch |
| 6 | Card management | Full virtual-card lifecycle in simulation | Stretch |

Core four are built first and tested before any stretch work begins.

## 3. Top-Level Structure

```
bankpilot/
├── PLAN.md                  ← this file (master index)
├── README.md                ← quickstart + entry point
├── docs/                    ← 11 supporting markdown files
│   ├── ARCHITECTURE.md
│   ├── DATA_MODEL.md
│   ├── API.md
│   ├── POLICY.md
│   ├── SECURITY.md
│   ├── SCENARIOS.md
│   ├── AGENT_PIPELINE.md
│   ├── DEMO_SCRIPT.md
│   ├── TESTING.md
│   ├── ROADMAP.md
│   └── THREAT_MODEL.md
├── backend/                 ← Rust + Actix Web (created in Build)
├── frontend/                ← Next.js 14 (created in Build)
├── db/                      ← SQLx migrations + seed (created in Build)
├── ops/                     ← env, scripts, reset (created in Build)
└── .env.example             ← DATABASE_URL, OPENAI_*, SESSION_SECRET
```

## 4. Phased Build Order

Each phase ends with a working demo slice and a checklist of tests passing. No phase is "done" until its acceptance criteria are green.

| Phase | Focus | Exit criteria | Doc anchor |
|---|---|---|---|
| **1. Foundation** | Repo, Rust API skeleton, Postgres migrations, seed, auth, role split | Login as customer + admin; seed accounts visible | `ROADMAP.md` §1 |
| **2. Security core** | Policy engine, pending actions, mock MFA, idempotency, audit hash chain, safety lock, daily limit | All Phase-2 security tests pass | `SECURITY.md`, `POLICY.md` |
| **3. Core scenarios** | Bill reports, transfer flow, subscription detection + mock cancel | Scenarios 1–3 end-to-end with real data | `SCENARIOS.md` |
| **4. Agent + DAG** | Intent pipeline, fixed tool allowlist, DAG executor, pause/cancel/compensate, birthday plan | Scenario 4 demo runnable in 60s | `AGENT_PIPELINE.md` |
| **5. Stretch (5+6)** | Wealth catalog/suitability/simulated fulfilment; virtual card lifecycle | Card issue→freeze→cancel works in sim | `SCENARIOS.md` §5–6 |
| **6. Frontend + demo** | Chat, dashboards, confirmations, timeline, admin console; states (loading/empty/error/denied) | All 8 screens render; demo script runs in 5 min | `DEMO_SCRIPT.md` |
| **7. Submission** | Full test suite, README, architecture diagram, bilingual security self-assessment, 5-min demo video | Deliverables checklist 100% | `ROADMAP.md` §7 |

The stretch phase (5) is gated on Phase 1–4 being green. If time is short, scenarios 5 and 6 stay read-only with a clear "simulation only" banner.

## 5. Hard Architectural Rules

These are non-negotiable and enforced by code review + tests:

1. **Money is integer minor units** (fen for CNY). Never `f64` for ledger math. See `DATA_MODEL.md`.
2. **The LLM is a planner, not an authority.** It proposes structured `proposed_actions[]`; the backend re-resolves every recipient, validates schemas, checks policy, and only then executes.
3. **The policy engine is the only place risk tier is decided.** Tier may escalate during a conversation; it cannot be downgraded.
4. **Yellow = one explicit confirmation; Red = confirmation + fresh mock MFA.** UI shows exact parameters; any change invalidates approval.
5. **All untrusted text** (memos, payee names, merchant names, file uploads, connector responses) is sandboxed — never re-interpreted as instructions.
6. **Append-only audit log** with hash chain. Sensitive fields masked. No secrets, OTPs, full account numbers, or passwords logged.
7. **Idempotency keys** on every state-changing tool call. Replays do not double-execute.
8. **Daily transfer limit** is enforced server-side, summed across all channels, not per-request.
9. **Safety lock** triggers on N failed MFA, denied high-risk actions, rapid high-value attempts, or injection patterns. Lock blocks Yellow/Red; requires admin unlock.
10. **Identity is derived from the session**, never from request body fields.

## 6. Component Map (mirrors `ARCHITECTURE.md`)

```
Frontend (Next.js)
  ├── Chat + lang selector
  ├── Spending dashboard (Recharts)
  ├── Plan preview / confirm / mock MFA
  ├── Subscription centre
  ├── Workflow timeline
  └── Admin console

Rust API (Actix Web)
  ├── auth/        session cookie + CSRF, Argon2, role gates
  ├── users/       demo identity
  ├── accounts/    masked accounts, balance, reserved
  ├── transactions/ ledger
  ├── payees/      mock contacts
  ├── transfers/   preview, confirm, execute
  ├── bills/       deterministic reports, anomaly rules
  ├── subscriptions/ detection, reminder, sim-cancel
  ├── reminders/   due dates
  ├── goals/       savings reservations
  ├── cards/       virtual card lifecycle (sim)
  ├── wealth/      product catalog, suitability, sim-fulfilment
  ├── agent/       message intake
  ├── planner/     DAG construction from LLM proposal
  ├── workflows/   executor, pause, cancel, compensation
  ├── policy/      Green/Yellow/Red decision
  ├── approvals/   pending-action store
  ├── mfa/         mock challenges
  ├── audit/       append-only events + hash chain
  ├── safety_lock/ failure counters + lock state
  ├── connectors/  mock merchant + reminder/calendar + flowers/cake
  └── errors/      typed error type, never panics across the wire

LLM Adapter
  ├── openai/      OpenAI-compatible client (chat completions + JSON mode)
  └── stub/        deterministic regex/heuristic intent parser (default if no key)

PostgreSQL
  users, accounts, transactions, payees, transfers, subscriptions,
  reminders, savings_goals, reservations, cards, wealth_products,
  wealth_holdings, workflows, workflow_nodes, pending_actions,
  audit_events, safety_events, sessions
```

## 7. Demo Endpoints (the 5-minute walkthrough maps here)

| Time | Beat | Backed by | Doc |
|---|---|---|---|
| 0:00–0:30 | Problem + simulation banner | `/` | `DEMO_SCRIPT.md` §1 |
| 0:30–1:20 | Bill analysis | `GET /api/v1/reports/spending`, `/anomalies` | `SCENARIOS.md` §2 |
| 1:20–2:20 | Smart transfer (mixed-lang) | `POST /api/v1/agent/messages` → `POST /api/v1/actions/{id}/confirm` | `SCENARIOS.md` §1 |
| 2:20–3:10 | Permission enforcement (Red) | `/actions/{id}/verify` (mock MFA) | `POLICY.md` |
| 3:10–4:10 | Birthday DAG | workflow creation, node confirmations | `AGENT_PIPELINE.md`, `SCENARIOS.md` §4 |
| 4:10–4:40 | Interrupt + compensate | `POST /api/v1/workflows/{id}/pause` | `AGENT_PIPELINE.md` §3 |
| 4:40–5:00 | Security conclusion | `GET /api/v1/audit`, injection test | `SECURITY.md` |

## 8. Risks Called Out Up Front

| Risk | Mitigation |
|---|---|
| LLM API unavailable / cost | Stub adapter; JSON-mode cache; demo path runs fully offline |
| 6-scenario scope creep | Stretch (5+6) gated on Phase 1–4 green; explicit "if time" |
| Coolify Postgres downtime | `DATABASE_URL` only; migrations idempotent; reset script |
| Time zone / locale bugs in CN prompts | Locale fixtures; mixed-language test corpus in `TESTING.md` |
| Audit hash-chain drift in tests | Reset script re-seeds chain from known seed event |

## 9. How to Use This Plan

- This file is the index. Every other doc lives under `docs/` and is linked from here.
- Each phase has a "Doc anchor" linking to the section that defines its acceptance criteria.
- Status changes (phase complete, scope change, new risk) are reflected in the matching `docs/` file, not here, to avoid drift.

## 10. Next Step

When you switch to **Build** mode, the agent will:
1. Create `backend/`, `frontend/`, `db/`, `ops/` skeletons (Cargo workspace, `package.json`, SQLx migrations dir).
2. Drop a working `.env.example` and a `Reset` script.
3. Begin Phase 1: Postgres migrations, seed, auth, role-gated `/me`.
4. Run the Phase 1 test gate before moving on.

See `ROADMAP.md` for the full phase breakdown.
