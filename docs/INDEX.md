# Documentation Index

> A flat, single-page table of contents for tools that don't render
> sidebars (CLI pagers, raw `cat`, search engines).

## Foundations

- [Architecture (legacy single file)](./ARCHITECTURE.md) —
  component map, request lifecycles, deployment topology.
- **Architecture (bilingual tree)** *(new — supersedes the legacy
  single file for new work)*:
  - [Overview](./architecture/README.md)
  - [0 · System Overview](./architecture/00-system-overview.md)
  - [1 · Request Lifecycle](./architecture/01-request-lifecycle.md)
  - [2 · Data Model](./architecture/02-data-model.md)
  - [3 · Core Algorithms](./architecture/03-algorithms.md)
  - [4 · Frontend](./architecture/04-frontend.md)
  - [5 · Deployment](./architecture/05-deployment.md)
  - [6 · Security Design](./architecture/06-security-design.md)
  - **架构总览 (中文)**:
    - [架构总览](./architecture/zh/README.md)
    - [0 · 系统总览](./architecture/zh/00-system-overview.md)
    - [1 · 请求生命周期](./architecture/zh/01-request-lifecycle.md)
    - [2 · 数据模型](./architecture/zh/02-data-model.md)
    - [3 · 核心算法](./architecture/zh/03-algorithms.md)
    - [4 · 前端](./architecture/zh/04-frontend.md)
    - [5 · 部署](./architecture/zh/05-deployment.md)
    - [6 · 安全设计](./architecture/zh/06-security-design.md)
- [Data Model](./DATA_MODEL.md) — entities, schema, money handling
  rules.
- [API Reference](./API.md) — REST endpoints, request/response
  schemas.
- [Policy Engine](./POLICY.md) — Green / Yellow / Red engine,
  escalation rules.

## Security

- [Security Design](./SECURITY.md) — internal: threat model,
  layered defenses, security test matrix.
- **Security Self-Assessment** *(external, bilingual)*:
  - [Overview](./self-assessment/README.md)
  - [0 · Scope](./self-assessment/00-scope.md)
  - [1 · Permission Tiers](./self-assessment/01-permission-tiers.md)
  - [2 · Audit & Observability](./self-assessment/02-audit.md)
  - [3 · LLM Safety Boundaries](./self-assessment/03-llm-safety.md)
  - [4 · Known Risks (R-1 … R-15)](./self-assessment/04-known-risks.md)
  - [5 · Non-Claims](./self-assessment/05-non-claims.md)
  - [6 · Code Map](./self-assessment/06-code-map.md)
  - **中文附录**:
    - [总览](./self-assessment/zh/README.md)
    - [0 · 范围](./self-assessment/zh/00-scope.md)
    - [1 · 权限分级](./self-assessment/zh/01-permission-tiers.md)
    - [2 · 审计](./self-assessment/zh/02-audit.md)
    - [3 · 大模型安全](./self-assessment/zh/03-llm-safety.md)
    - [4 · 已知风险](./self-assessment/zh/04-known-risks.md)
    - [5 · 未声称](./self-assessment/zh/05-non-claims.md)
    - [6 · 代码索引](./self-assessment/zh/06-code-map.md)

## Agent & Demos

- [Scenarios](./SCENARIOS.md) — the 6 user scenarios, demo
  prompts.
- **Operations & Testing** *(new — deployment + tests)*:
  - [Overview](./operations/README.md)
  - [0 · Deployment](./operations/00-deployment.md) — production
    topology, systemd units, reverse proxy, Postgres ops, backups,
    upgrade & rollback, log rotation.
  - [1 · Troubleshooting](./operations/01-troubleshooting.md) —
    symptom → log line → recovery.
  - [2 · Test Cases](./operations/02-test-cases.md) — 39 unit
    tests enumerated, manual integration checklist, red-team
    scenarios, smoke script, coverage gaps.
  - [3 · CI](./operations/03-ci.md) — recommended GitHub Actions
    pipeline (none today); PR gates, smoke job on `main`.
  - **运维与测试 (中文)**:
    - [总览](./operations/zh/README.md)
    - [0 · 部署](./operations/zh/00-deployment.md)
    - [1 · 故障排查](./operations/zh/01-troubleshooting.md)
    - [2 · 测试用例](./operations/zh/02-test-cases.md)
    - [3 · 持续集成](./operations/zh/03-ci.md)

## Top-level

- [Docs Home](./README.md) — landing page with curated reading
  order.
- [Sidebar](./_sidebar.md) — Docusaurus / Docsify sidebar definition.

---

## How this is organised

```
docs/
├── README.md                      ← landing page
├── INDEX.md                       ← this file
├── _sidebar.md                    ← sidebar for SSG importers
│
├── ARCHITECTURE.md                ← foundations (legacy single file)
├── architecture/                  ← bilingual architecture tree
│   ├── README.md                  ← landing for the tree
│   ├── 00-system-overview.md
│   ├── 01-request-lifecycle.md
│   ├── 02-data-model.md
│   ├── 03-algorithms.md
│   ├── 04-frontend.md
│   ├── 05-deployment.md
│   ├── 06-security-design.md
│   └── zh/                        ← 中文镜像,7 页
│       ├── README.md
│       ├── 00-system-overview.md
│       ├── 01-request-lifecycle.md
│       ├── 02-data-model.md
│       ├── 03-algorithms.md
│       ├── 04-frontend.md
│       ├── 05-deployment.md
│       └── 06-security-design.md
├── DATA_MODEL.md
├── API.md
├── POLICY.md
│
├── SECURITY.md                    ← internal security design
│
├── self-assessment/               ← external self-assessment
│   ├── README.md                  ← overview + reading order
│   ├── 00-scope.md
│   ├── 01-permission-tiers.md
│   ├── 02-audit.md
│   ├── 03-llm-safety.md
│   ├── 04-known-risks.md
│   ├── 05-non-claims.md
│   ├── 06-code-map.md
│   ├── 07-banking-api-safety.md
│   └── zh/                        ← 中文镜像,8 页
│       ├── README.md
│       ├── 00-scope.md
│       ├── 01-permission-tiers.md
│       ├── 02-audit.md
│       ├── 03-llm-safety.md
│       ├── 04-known-risks.md
│       ├── 05-non-claims.md
│       ├── 06-code-map.md
│       └── 07-banking-api-safety.md
│
├── operations/                    ← deployment + tests (bilingual)
│   ├── README.md
│   ├── 00-deployment.md
│   ├── 01-troubleshooting.md
│   ├── 02-test-cases.md
│   ├── 03-ci.md
│   └── zh/                        ← 中文镜像,5 页
│       ├── README.md
│       ├── 00-deployment.md
│       ├── 01-troubleshooting.md
│       ├── 02-test-cases.md
│       └── 03-ci.md
│
└── SCENARIOS.md                   ← demo prompts
```

Every page has YAML front-matter (`id`, `title`, `sidebar_label`,
`sidebar_position`) so Docusaurus, Mintlify, Nextra, or Docsify can
import the folder with no extra work.
