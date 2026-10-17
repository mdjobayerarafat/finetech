---
id: docs-home
title: BankPilot Docs
sidebar_label: Home
sidebar_position: 0
---

# BankPilot Docs

> Secure, multilingual AI banking agent — simulation only, no real money.

This is the technical documentation for BankPilot. Pick a section:

## Foundations

| Doc | What it covers |
|---|---|
| [`ARCHITECTURE.md`](./ARCHITECTURE.md) | Component map, request lifecycles, deployment. |
| [`DATA_MODEL.md`](./DATA_MODEL.md) | Entities, schema, money handling. |
| [`API.md`](./API.md) | REST endpoints, request/response schemas. |
| [`POLICY.md`](./POLICY.md) | Green / Yellow / Red engine, escalation. |

## Security

| Doc | What it covers |
|---|---|
| [`SECURITY.md`](./SECURITY.md) | Threat model, defenses, security test matrix. |
| **[`self-assessment/`](./self-assessment/README.md)** | **External self-assessment for reviewers** (Phase 7 deliverable, bilingual). |

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

- **New to the project?** Read [`ARCHITECTURE.md`](./ARCHITECTURE.md),
  then [`DATA_MODEL.md`](./DATA_MODEL.md).
- **Reviewing security?** Start at
  [`SECURITY.md`](./SECURITY.md) (defenses) and
  [`self-assessment/`](./self-assessment/README.md) (the honest list
  of risks). The two are complementary, not redundant.
- **Demoing?** `SCENARIOS.md` has the prompts; the demo script
  (when written) will have the timing.
