---
id: scope
title: "0 · Scope and what this is not"
sidebar_label: "0 · Scope"
sidebar_position: 1
---

# 0. Scope and what this is not

> 中文: [00-scope.md (中文)](./zh/00-scope.md)

## In scope

- **Permission-tier implementation** — the Green / Yellow / Red
  decision flow, escalation rules, and how each tool is classified.
- **Daily-limit and `forbid_tool` mechanisms** — admin-configurable
  rules evaluated in the planner.
- **Audit hash chain** — what is written, how it is verified, what
  the integrity endpoint exposes.
- **Mock-MFA verification** — the in-process `MFA_DEMO_CODE` path
  and its limitations.
- **Session and CSRF handling** — cookie attributes, the
  `X-CSRF-Token` envelope, the safety lock.
- **Prompt-injection defenses** — the fixed system prompt, the
  structured `proposed_actions` envelope, server-side
  argument re-resolution.
- **Known risks and the residual attack surface** — the honest list
  (R-1 through R-15) of what is *not* defended today.

## Out of scope

- General product UX, demo-script polish, the language model's
  qualitative answers.
- The full STRIDE-style threat model — that lives in
  [`../THREAT_MODEL.md`](../THREAT_MODEL.md) (referenced; not yet
  written in this build).
- Performance, scaling, and cost analysis.

## Important context for reviewers

FinTech Pioneers is a **demonstration platform**. No real banks, no real
cards, no real money. Every "transfer" is a row in a Postgres
ledger; every "card" is a tuple in `cards`. The security controls
in this document are written to defend that ledger *as if it were
real*, but several cryptographic and operational primitives are
intentionally simplified so the demo runs without a vault, an HSM,
or an SMS provider. Each of those is named explicitly in
[§4 — Known risks](./04-known-risks.md).

> This context is the framing for every claim below. A risk
> labelled "demo-acceptable" means *acceptable for a built-fresh,
> controlled demo*; a risk labelled "production-unacceptable"
> means *would have to close before this carried real customer
> funds*.

## How to read this

Each page has the same shape:

- A short framing paragraph.
- Tables and code references where helpful.
- For risks: **Risk → Status → Hardening path** triples, in that
  order, so a reader can decide for themselves whether the trade-off
  is acceptable for the deployment they're imagining.
