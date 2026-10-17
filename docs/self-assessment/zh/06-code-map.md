---
id: code-map-zh
title: "6 · 代码中看哪里"
sidebar_label: "6 · 代码索引"
sidebar_position: 7
---

# 6. 代码中看哪里

> English: [06-code-map.md (EN)](../06-code-map.md)

§1–§4 的每条主张都对应到具体的文件与函数。本页是索引。

| 关注点 | 文件 / 函数 |
|---|---|
| 等级常量 | `backendrust/src/policy.rs` (`GREEN`、`YELLOW`、`RED`) |
| 每个工具的等级决策 | `backendrust/src/policy.rs::decide_*` |
| 规划器、升级、补偿 | `backendrust/src/planner.rs::decide_node` |
| 工具 spec + 槽位校验 | `backendrust/src/tool_spec.rs` |
| 工具注册表 + 规范化名 | `backendrust/src/tools.rs::registry` / `canonicalize_tool_name` |
| `forbid_tool` / `restrict_daily_fen` | `backendrust/src/agent_rules.rs` |
| 安全锁、MFA 失败 | `backendrust/src/safety.rs` |
| 审计追加 + 链校验 | `backendrust/src/audit.rs` |
| 待处理动作生命周期 | `backendrust/src/routes/actions.rs` |
| 智能体路由(LLM → 规划器 → 执行) | `backendrust/src/routes/agent.rs` |
| 优化报告(用户范围) | `backendrust/src/routes/accounts.rs::report_optimization` |
| 预算 / 预留金流 | `backendrust/src/routes/budgets.rs`、`tools::t_budget_*` |
| 大模型桩回退 | `backendrust/src/llm.rs::stub_propose` |
| 前端 CSRF 封装 | `frontend/src/lib/api.ts::request` |
| 前端确认卡 | `frontend/src/app/chat/page.tsx::ConfirmationCard` |
| 管理员规则 UI | `frontend/src/app/admin/` |

**返回:** [总览 →](./README.md)
