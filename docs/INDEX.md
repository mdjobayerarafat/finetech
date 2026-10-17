# Documentation Index

> A flat, single-page table of contents for tools that don't render
> sidebars (CLI pagers, raw `cat`, search engines).

## Foundations

- [Architecture](./ARCHITECTURE.md) — component map, request
  lifecycles, deployment topology.
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

- [Scenarios](./SCENARIOS.md) — the 6 user scenarios, demo prompts.

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
├── ARCHITECTURE.md                ← foundations
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
│   └── zh/                        ← 中文镜像,同样 7 页
│       ├── README.md
│       ├── 00-scope.md
│       ├── 01-permission-tiers.md
│       ├── 02-audit.md
│       ├── 03-llm-safety.md
│       ├── 04-known-risks.md
│       ├── 05-non-claims.md
│       └── 06-code-map.md
│
└── SCENARIOS.md                   ← demo prompts
```

Every page has YAML front-matter (`id`, `title`, `sidebar_label`,
`sidebar_position`) so Docusaurus, Mintlify, Nextra, or Docsify can
import the folder with no extra work.
