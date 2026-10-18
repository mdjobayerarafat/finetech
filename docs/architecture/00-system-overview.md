---
id: arch-system-overview
title: "0 · System Overview"
sidebar_label: "0 · System Overview"
sidebar_position: 1
---

# 0. System Overview

> 中文: [00-系统总览 (中文)](./zh/00-system-overview.md)

FinTech Pioneers is a simulation-only, bilingual AI banking agent. The
**user** chats, asks, or fills a form; the **agent** proposes an
action; the **policy engine** decides whether it is
Green / Yellow / Red; the **executor** either runs it or queues it
for a single-tap / MFA confirmation. **Money never moves outside
the simulator.**

The whole project lives in this repository:

```
bankpilot/
├── backendrust/       Rust + Actix Web API, SQLx, LLM adapter, seed
├── frontend/          Next.js 14 (App Router), React 18, Tailwind, i18n
├── docs/              This documentation tree
├── ops/dev.sh         One-command dev stack
├── .env               Runtime configuration (loaded by ops/dev.sh)
├── .env.example       Template for a fresh checkout
├── PLAN.md            High-level product spec
├── package.json       Root pnpm workspace marker
├── pnpm-lock.yaml
└── FinTech Pioneers_Project_Details_and_Feature_Specification.docx
```

## 0.1 Component map

```mermaid
flowchart TB
    %% --- Browser surface ---------------------------------------------------
    subgraph Browser["Browser — Next.js 14"]
        direction TB
        UI["App Router pages<br/>(dashboard, chat, transfers, …)"]
        I18N["LanguageProvider<br/>(en / zh)"]
        Bus["lib/store.ts<br/>(cross-page bus)"]
        UI --- I18N
        UI --- Bus
    end

    %% --- Rust backend -----------------------------------------------------
    subgraph API["Rust API — Actix Web · :8080"]
        direction TB
        Auth["routes/auth.rs<br/>session + CSRF"]
        Agent["routes/agent.rs<br/>chat handler"]
        Tools["routes/tools.rs<br/>REST tools surface"]
        Admin["routes/admin.rs<br/>rules + audit"]

        Planner["planner.rs<br/>DAG + tier decision"]
        Tools2["tools.rs<br/>typed registry"]
        Specs["tool_spec.rs<br/>slot metadata"]
        Policy["policy.rs<br/>Green/Yellow/Red"]
        Fraud["fraud.rs<br/>scam guard"]
        Rules["agent_rules.rs<br/>admin blocklist"]
        Forecast["forecast.rs<br/>what-if projection"]
        Receipt["receipt.rs<br/>OCR stub"]
        Smart["smart_transfers.rs<br/>payee resolve + parse"]
        Ret["retriever.rs / memory.rs<br/>conversation memory"]
        Audit["audit.rs<br/>hash-chained events"]

        Auth --> Planner
        Agent --> Planner
        Tools --> Tools2
        Admin --> Rules
        Planner --> Policy
        Planner --> Tools2
        Planner --> Fraud
        Planner --> Rules
        Tools2 --> Specs
        Agent --> Ret
        Smart --> Planner
    end

    %% --- External --------------------------------------------------------
    subgraph LLM["LLM provider<br/>(OpenAI-compatible / Unsloth)"]
        LLMNode["POST /chat/completions<br/>temperature 0.2–0.4"]
    end
    subgraph Mock["Mock connectors<br/>(bank sandbox)"]
        Merchant["merchant"]
        Calendar["calendar"]
        Flowers["flowers"]
        Cake["cake"]
    end
    subgraph DB["PostgreSQL"]
        Tables[("users · accounts<br/>transactions · payees<br/>transfers · subscriptions<br/>reminders · savings_goals<br/>reservations · cards<br/>wealth_* · budgets<br/>budget_reservations<br/>agent_rules · audit_events<br/>safety_events · pending_actions<br/>workflows · workflow_nodes<br/>conversations · conversation_messages<br/>split_bills · split_bill_parts")]
    end

    %% --- Wires -----------------------------------------------------------
    UI -- "session cookie + CSRF<br/>JSON body" --> Agent
    UI -- "REST" --> Tools
    UI -- "REST (admin)" --> Admin
    UI -- "POST /auth/login" --> Auth
    Planner -. "LLM proposal" .-> LLMNode
    LLMNode -. "JSON plan" .-> Planner
    Tools2 -- "side-effects" --> Mock
    Planner -- "txn writes" --> DB
    Tools2 -- "txn writes" --> DB
    Audit -- "append only" --> DB
    Smart -- "txn writes" --> DB
    Forecast -- "read-only" --> DB
```

> **Key.** Solid arrows = synchronous request paths. Dotted arrows
> = outbound calls to the LLM provider. The `DB` block lists every
> production table — see [02-data-model.md](./02-data-model.md) for
> the ER diagram.

## 0.2 What runs where

| Layer | Tech | Port | Source |
|---|---|---|---|
| Frontend | Next.js 14 (App Router) + React 18 + Tailwind | `:3000` | `frontend/` |
| Backend | Rust 2021 + Actix Web 4 + SQLx | `:8080` | `backendrust/` |
| Database | PostgreSQL (external, Coolify) | `:5432` | `DATABASE_URL` |
| LLM provider | OpenAI-compatible HTTP (Unsloth by default) | external | `UNSLOTH_URL`, `UNSLOTH_API_KEY`, `UNSLOTH_MODEL` |
| Mock connectors | In-process Rust functions, no network | — | `backendrust/src/routes/features.rs` and tool registry |

Postgres is **not** packaged in this repo. The dev script points
`DATABASE_URL` at a remote instance (Coolify in production,
`postgres@127.0.0.1` in local dev). Migrations are plain SQL under
`backendrust/migrations/`.

## 0.3 Deployment topology

```mermaid
flowchart LR
    Dev["Developer laptop<br/>./ops/dev.sh"] -- "cargo build<br/>next dev" --> Frontend["Next.js<br/>:3000"]
    Dev --> Backend["Rust API<br/>:8080"]
    Frontend -- "HTTP / same-origin" --> Backend
    Backend -- "SQLx" --> DB[("PostgreSQL<br/>(Coolify)")]
    Backend -- "HTTPS<br/>/chat/completions" --> LLM[("LLM provider<br/>(Unsloth)")]
    Browser["Browser<br/>(end user)"] -- "session cookie" --> Frontend

    classDef ext fill:#fef3c7,stroke:#b45309;
    class DB,LLM ext;
```

> Production differs only in that `BACKEND_PORT` and the Postgres
> host move behind a reverse proxy. There is no built-in Docker
> compose — the assumption is the operator already has Postgres
> reachable.

## 0.4 Module responsibilities (high level)

| Module | Responsibility | Source |
|---|---|---|
| `auth` | session cookie, CSRF token, password hashing, safety-lock state | `routes/auth.rs`, `sessions.rs`, `security.rs` |
| `agent` | the chat endpoint: LLM propose → plan → execute | `routes/agent.rs`, `planner.rs` |
| `tools` | the typed registry of side-effectful operations | `tools.rs`, `tool_spec.rs` |
| `policy` | Green / Yellow / Red decision per action | `policy.rs` |
| `fraud` | scam-pattern scoring, narrative generation | `fraud.rs` |
| `agent_rules` | admin-configurable block / restrict / prefix rules | `agent_rules.rs` |
| `forecast` | "can I afford it?" projection | `forecast.rs` |
| `receipt` | receipt/invoice → draft (stub or LLM) | `receipt.rs` |
| `smart_transfers` | payee resolve, free-form transfer parsing, split-bills | `smart_transfers.rs` |
| `planner` | DAG construction, slot filling, escalation, compensation | `planner.rs` |
| `retriever` / `memory` | conversation memory + RAG retriever | `retriever.rs`, `memory.rs` |
| `audit` | append-only hash-chained event log | `audit.rs` |
| `safety` | safety lock state machine, MFA failure backoff | `safety.rs` |
| `llm_http` | single HTTP client, timeouts, retry | `llm_http.rs` |
| `llm` | LLM adapter (provider → Proposal), with regex stub fallback | `llm.rs` |
| `translate` | detector + per-language wording tables for the chat reply | `translate.rs` |
| `seed` | idempotent demo data loader | `seed.rs` |

## 0.5 Where to go next

- Want to follow a single request? → [01-request-lifecycle.md](./01-request-lifecycle.md)
- Want the schema? → [02-data-model.md](./02-data-model.md)
- Want the algorithms? → [03-algorithms.md](./03-algorithms.md)
- Want the frontend tree? → [04-frontend.md](./04-frontend.md)
- Want the dev workflow? → [05-deployment.md](./05-deployment.md)
- Want the security layers? → [06-security-design.md](./06-security-design.md)
