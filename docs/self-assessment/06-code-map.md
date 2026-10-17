---
id: code-map
title: "6 · Where to look in the code"
sidebar_label: "6 · Code map"
sidebar_position: 7
---

# 6. Where to look in the code

> 中文: [06-code-map.md (中文)](./zh/06-code-map.md)

Every claim in §1–§4 maps to a concrete file and function. This
page is the index.

| Concern | File / function |
|---|---|
| Tier constants | `backendrust/src/policy.rs` (`GREEN`, `YELLOW`, `RED`) |
| Per-tool tier decision | `backendrust/src/policy.rs::decide_*` |
| Planner, escalation, compensation | `backendrust/src/planner.rs::decide_node` |
| Tool specs + slot validation | `backendrust/src/tool_spec.rs` |
| Tool registry + canonicalize | `backendrust/src/tools.rs::registry` / `canonicalize_tool_name` |
| `forbid_tool` / `restrict_daily_fen` | `backendrust/src/agent_rules.rs` |
| Safety lock, MFA failures | `backendrust/src/safety.rs` |
| Audit append + chain verify | `backendrust/src/audit.rs` |
| Pending action lifecycle | `backendrust/src/routes/actions.rs` |
| Agent route (LLM → planner → execute) | `backendrust/src/routes/agent.rs` |
| Optimization report (user-scoped) | `backendrust/src/routes/accounts.rs::report_optimization` |
| Budget / reservation flow | `backendrust/src/routes/budgets.rs`, `tools::t_budget_*` |
| LLM stub fallback | `backendrust/src/llm.rs::stub_propose` |
| Frontend CSRF wrapper | `frontend/src/lib/api.ts::request` |
| Frontend confirmation card | `frontend/src/app/chat/page.tsx::ConfirmationCard` |
| Admin rule UI | `frontend/src/app/admin/` |

**Back to:** [Overview →](./README.md)
