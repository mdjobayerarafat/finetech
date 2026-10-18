---
id: arch-home
title: FinTech Pioneers Architecture
sidebar_label: Architecture Home
sidebar_position: 0
---

# FinTech Pioneers Architecture (Bilingual)

> 中文版: [架构总览 (中文)](./zh/README.md)

This folder is the **internal** design reference for FinTech Pioneers. It
complements — but does not replace — the [legacy single-file
architecture note](../ARCHITECTURE.md) and the
[external self-assessment](../self-assessment/README.md). Where the
self-assessment describes *what is and is not safe to claim*, this
tree describes *how the system is built*.

The pages are designed to be read in order. Each one assumes the
previous.

| # | Page | What it covers |
|---|---|---|
| 0 | [System Overview](./00-system-overview.md) | Component map, deployment topology, a Mermaid `flowchart` of the whole stack, and a Mermaid deployment view. |
| 1 | [Request Lifecycle](./01-request-lifecycle.md) | The four stages every agent request goes through (Plan → Validate → Authorise → Execute), a `sequenceDiagram` of a Yellow-tier transfer, and a `stateDiagram-v2` of the LLM tool loop. |
| 2 | [Data Model](./02-data-model.md) | `erDiagram` of the 16 production tables, money-handling rules (`*_fen` BIGINT), and the `audit_events` hash chain. |
| 3 | [Core Algorithms](./03-algorithms.md) | Pseudocode and source map for the seven non-trivial algorithms: policy tier, fraud guard, smart-transfer parsing, "can I afford it?" forecast, receipt OCR, planner DAG, and subscription detection. |
| 4 | [Frontend](./04-frontend.md) | Next.js 14 App Router tree, bilingual i18n implementation, shared component libraries, the `bus` cross-page refresh pattern. |
| 5 | [Deployment](./05-deployment.md) | `ops/dev.sh` workflow, environment variables, build/run commands, log layout under `.run/`. |
| 6 | [Security Design](./06-security-design.md) | The eight layers of defence in depth, mapping every claim to a source file. Cross-links to the [self-assessment](../self-assessment/README.md) for risk and non-claim pages. |

## Who this is for

- **Engineers** onboarding to the Rust backend or the Next.js
  frontend.
- **Reviewers** who need to verify a security or architecture claim
  against concrete code.
- **Demo operators** who want to understand which log lines mean what
  when a Yellow tier pops up on the chat screen.

## How to read it offline

Every page has YAML front-matter (`id`, `title`, `sidebar_label`,
`sidebar_position`) so Docusaurus, Mintlify, Nextra, or Docsify can
import the folder. The Mermaid diagrams are inline ` ```mermaid `
fences — GitHub renders them natively; Docusaurus needs
`@docusaurus/theme-mermaid`.

## What is **not** here

- The **legacy single-file architecture note** ([../ARCHITECTURE.md](../ARCHITECTURE.md)) — kept for history; superseded for new work by this tree.
- The **external security self-assessment** ([../self-assessment/](../self-assessment/README.md)) — bilingual, reviewer-facing; covers risks and non-claims.
- **API endpoint reference** — see [../API.md](../API.md).
- **Per-tool tier reasoning** — see [../POLICY.md](../POLICY.md) and the self-assessment's [§1 Permission Tiers](../self-assessment/01-permission-tiers.md).

## Mirror

- **English (canonical)**: this folder.
- **中文镜像**: [./zh/](./zh/) — every page in this tree is mirrored.
