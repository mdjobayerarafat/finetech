---
id: ops-home
title: Operations & Testing
sidebar_label: Operations Home
sidebar_position: 0
---

# Operations & Testing

> 中文版: [运维与测试 (中文)](./zh/README.md)

This folder is the **run-the-system** reference for FinTech
Pioneers. It complements — but does not replace — the
[architecture tree](../architecture/README.md) and the
[external self-assessment](../self-assessment/README.md).

| # | Page | What it covers |
|---|---|---|
| 0 | [Deployment](./00-deployment.md) | Production topology, environment variables, `systemd` units, reverse proxy + TLS, Postgres operations (migrations, backups, reset), upgrade and rollback, log rotation, observability. |
| 1 | [Troubleshooting](./01-troubleshooting.md) | Common failure modes, what each log line means, recovery commands, when to escalate. |
| 2 | [Test Cases](./02-test-cases.md) | All 39 unit tests enumerated, manual integration checklist, redteam scenarios, end-to-end smoke scripts, the 6 user scenarios from `SCENARIOS.md` cross-linked. |
| 3 | [CI](./03-ci.md) | Recommended CI pipeline (none today), GitHub Actions template, what to gate on, what to skip. |

## Who this is for

- **Operators** who stand up the production environment, run
  migrations, rotate secrets, and answer the pager.
- **Reviewers** who want to see the test surface and how each
  security claim in the self-assessment is exercised.
- **Demo runners** who need a deterministic quickstart and a
  recovery path when the in-flight Next.js / Rust processes die
  mid-demo.

## How to read it

Each page is self-contained. The cross-link that matters most is
**`02-test-cases.md` ↔ `../self-assessment/`** — every claim in
the self-assessment should be traceable to a test or a manual
checklist item here.

## What is **not** here

- **The architecture tree** — [../architecture/](../architecture/README.md). Read first if you don't know how the system is built.
- **The external self-assessment** — [../self-assessment/](../self-assessment/README.md). Read first if you need the risk/non-claim list.
- **Demo script** — see `../SCENARIOS.md` for the six user
  scenarios, and the in-app `/docs` page once a build is up.
- **The single-file legacy architecture note** — [../ARCHITECTURE.md](../ARCHITECTURE.md), kept for history.
