---
id: non-claims
title: "5 · What we are not claiming"
sidebar_label: "5 · Non-claims"
sidebar_position: 6
---

# 5. What we are not claiming

> 中文: [05-non-claims.md (中文)](./zh/05-non-claims.md)

- We are **not** claiming the platform is production-ready. Every
  entry in [§4](./04-known-risks.md) would have to close before
  this carried real money.
- We are **not** claiming the LLM is safe. The red-team suite is
  a regression net, not a proof.
- We are **not** claiming the audit log is tamper-evident in the
  cryptographic sense. [R-2](./04-known-risks.md#r-2-no-real-key-custody)
  and [R-12](./04-known-risks.md#r-12-the-audit-table-is-mutable-in-sql-no-db-level-guard)
  spell out the gap.
- We are **not** claiming the cross-scenario orchestration is
  novel — it's a planner + a tool allowlist + a savings goal + a
  reminder. What's novel is the integration: the same plumbing
  that makes a transfer pause for MFA also makes a birthday
  reservation pause for MFA.

**Next:** [§6 — Code map →](./06-code-map.md)
