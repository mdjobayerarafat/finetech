---
id: index
title: Security Self-Assessment
sidebar_label: Overview
sidebar_position: 0
---

# Security Self-Assessment

> **Phase 7 deliverable.** This is the *external* self-assessment for
> reviewers and judges. The internal design doc — threat model, layered
> defenses, the "AI proposes, software decides" principle — is
> [`../SECURITY.md`](../SECURITY.md). Read that first for the receipts;
> read this for an honest list of what the system does today and what
> would have to harden before it carried real money.
>
> Bilingual: **English** first; [中文附录](./zh/README.md) at the end
> of each page. Both versions cover the same material.

## What this is

A 7-section walk through:

1. The **permission-tier** implementation (Green / Yellow / Red).
2. The **audit and observability** story (hash chain, what the log
   holds, red-team suite).
3. The **LLM safety boundaries** (the architectural rules that keep
   the model from being authoritative).
4. **15 known risks** (R-1 through R-15), each with a *demo-acceptable
   / production-unacceptable* verdict and a concrete hardening path.
5. What we are **not** claiming.
6. Where in the **code** to verify each claim.

## Pages

| # | Topic | EN | 中文 |
|---|---|---|---|
| 0 | Scope and what this is not | [00-scope.md](./00-scope.md) | [00-scope.md](./zh/00-scope.md) |
| 1 | Permission tiers | [01-permission-tiers.md](./01-permission-tiers.md) | [01-permission-tiers.md](./zh/01-permission-tiers.md) |
| 2 | Audit and observability | [02-audit.md](./02-audit.md) | [02-audit.md](./zh/02-audit.md) |
| 3 | LLM safety boundaries | [03-llm-safety.md](./03-llm-safety.md) | [03-llm-safety.md](./zh/03-llm-safety.md) |
| 4 | Known risks (R-1 … R-15) | [04-known-risks.md](./04-known-risks.md) | [04-known-risks.md](./zh/04-known-risks.md) |
| 5 | What we are not claiming | [05-non-claims.md](./05-non-claims.md) | [05-non-claims.md](./zh/05-non-claims.md) |
| 6 | Code map | [06-code-map.md](./06-code-map.md) | [06-code-map.md](./zh/06-code-map.md) |

## Quick orientation for reviewers

- **Five minutes:** read [§0](./00-scope.md), skim the **R-1 … R-15**
  list in [§4](./04-known-risks.md), and the [code map](./06-code-map.md).
- **Fifteen minutes:** add [§1](./01-permission-tiers.md) and
  [§2](./02-audit.md) — the two sections that justify the "AI proposes,
  software decides" claim.
- **Deep read:** [§3](./03-llm-safety.md) for the LLM-boundary rules
  and the red-team suite, then the full [§4](./04-known-risks.md)
  with the hardening paths.

## Related docs

- [`../SECURITY.md`](../SECURITY.md) — internal design, threat model,
  layered defenses.
- [`../THREAT_MODEL.md`](../THREAT_MODEL.md) — STRIDE-style analysis
  (referenced; not yet written in this build).
- [`../POLICY.md`](../POLICY.md) — the Green / Yellow / Red engine
  in detail.
- [`../AGENT_PIPELINE.md`](../AGENT_PIPELINE.md) — how an intent
  becomes a plan (referenced).
