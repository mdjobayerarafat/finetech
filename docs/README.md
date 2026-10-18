---
id: docs-home
title: FinTech Pioneers Docs
sidebar_label: Home
sidebar_position: 0
---

# FinTech Pioneers Docs

> Secure, multilingual AI banking agent — simulation only, no real money.

This is the technical documentation for FinTech Pioneers. Pick a section:

## Foundations

| Doc | What it covers |
|---|---|
| [`ARCHITECTURE.md`](./ARCHITECTURE.md) | Legacy single-file architecture note (kept for history). |
| **[`architecture/`](./architecture/README.md)** | **Bilingual architecture tree** *(new — supersedes the legacy single file for new work)*. Component map, request lifecycle, data model, core algorithms, frontend, deployment, security design. Includes Mermaid system / sequence / state / ER diagrams. |
| [`DATA_MODEL.md`](./DATA_MODEL.md) | Entities, schema, money handling. |
| [`API.md`](./API.md) | REST endpoints, request/response schemas. |
| [`POLICY.md`](./POLICY.md) | Green / Yellow / Red engine, escalation. |

## Security

| Doc | What it covers |
|---|---|
| [`SECURITY.md`](./SECURITY.md) | Threat model, defenses, security test matrix. |
| **[`self-assessment/`](./self-assessment/README.md)** | **External self-assessment for reviewers** (Phase 7 deliverable, bilingual). |

## Operations & Testing *(new)*

| Doc | What it covers |
|---|---|
| **[`operations/`](./operations/README.md)** | **Deployment + tests** *(new)*. Production topology, systemd + nginx units, Postgres ops, backups, upgrade & rollback, troubleshooting, all 39 unit tests enumerated, manual integration checklist, red-team scenarios, smoke script, coverage gaps, recommended CI pipeline. Bilingual. |

## Agent & demos

| Doc | What it covers |
|---|---|
| `AGENT_PIPELINE.md` *(coming)* | Intent → slots → DAG → execution. |
| `THREAT_MODEL.md` *(coming)* | STRIDE-style analysis, mitigations. |
| `SCENARIOS.md` | The 6 user scenarios, demo prompts. |
| `TESTING.md` *(coming)* | Functional + security test plan. |
| `ROADMAP.md` *(coming)* | 7-phase build order, exit criteria. |
| `DEMO_SCRIPT.md` *(coming)* | 5-minute walkthrough for judges. |

## About this docs site

The docs are plain Markdown with Docusaurus / Mintlify / Docsify
front-matter so they can be imported into any static-site generator
later. Until then, every page renders in:

- **GitHub / GitLab** — the canonical viewer.
- **VS Code** — with a Markdown preview extension.
- **Obsidian** — for note-taking workflows.
- **Any CLI** — `cat`, `glow`, `mdcat`.

The top-level sidebar lives in [`_sidebar.md`](./_sidebar.md). The
table of contents is in [`INDEX.md`](./INDEX.md).

## Where to start

- **New to the project?** Read the new
  [`architecture/` tree](./architecture/README.md) (start with
  [0 · System Overview](./architecture/00-system-overview.md)),
  then [`DATA_MODEL.md`](./DATA_MODEL.md). The legacy
  [`ARCHITECTURE.md`](./ARCHITECTURE.md) is kept for history.
- **Reviewing security?** Start at
  [`architecture/06-security-design.md`](./architecture/06-security-design.md)
  (the *design* — eight layers of defence) and
  [`self-assessment/`](./self-assessment/README.md) (the *honest
  list* of risks and non-claims). The two are complementary, not
  redundant.
- **Operating or testing?** [`operations/`](./operations/README.md)
  has the deployment guide, troubleshooting matrix, 39 unit
  tests enumerated, manual integration checklist, red-team
  scenarios, smoke script, coverage gaps, and the recommended
  CI pipeline.
- **Demoing?** `SCENARIOS.md` has the prompts; the demo script
  (when written) will have the timing.
