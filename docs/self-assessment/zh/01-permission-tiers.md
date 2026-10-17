---
id: permission-tiers-zh
title: "1 · 权限分级"
sidebar_label: "1 · 权限分级"
sidebar_position: 2
---

# 1. 权限分级

> English: [01-permission-tiers.md (EN)](../01-permission-tiers.md)

## 1.1 三个等级

智能体可执行的每一个会变更状态的动作都按三个风险等级之一分类,
定义见 `backendrust/src/policy.rs`:

| 等级 | 常量 | 含义 |
|---|---|---|
| **绿色 (Green)** | `policy::GREEN` | 只读或可轻易撤销。模式与参数校验通过后自动放行。 |
| **黄色 (Yellow)** | `policy::YELLOW` | 可撤销但有影响。自动放行,但在 UI 上明确呈现。 |
| **红色 (Red)** | `policy::RED` | 不可撤销或涉及资金。始终需要用户明确确认; 配置了 MFA 还会强制校验。 |

**单次会话内等级单调上升**——节点可以从 Yellow 升到 Red,但
不能降级。这一点由 `planner::decide_node` 强制保证:代码里
没有 `tier: downgraded` 这样的路径。

## 1.2 等级如何决定

按权威性顺序,等级判定分散在三个文件中:

1. **`policy::decide_*` 函数** (`src/policy.rs`) — 权威来源。
   每个会变更状态的工具都有一个决策函数,返回
   `Decision { tier, reason, escalate_from, flags }`。
   - `decide_transfer` 检查金额、收款方验证状态、目标账户、
     每日累计总额、近期高风险行为。在达到每日阈值或
     连续被拒后可以从 Yellow 升级到 Red。
   - `decide_card_action("freeze")` → Yellow;
     `decide_card_action("cancel")` → Yellow 但带
     `confirm_required` 标记。
   - `decide_goal_reservation()` → Yellow;
     新增的 `decide_budget_reserve()` → Red(因为它实际把
     `available_balance_fen` 转到了 `reserved_balance_fen`)。
2. **`src/tool_spec.rs` 中每个工具的 spec** — 声明没有专门
   决策函数的工具的 `default_tier`
   (例如 `reminders.create` → Green,
   `subscriptions.cancel` → Yellow)。
3. **Postgres 中管理员配置的 `agent_rules`**
   (`agent_rules::forbidden_tools`、`restrict_amount_fen`、
   `restrict_daily_fen`) — 在策略决策*之后*评估。
   `forbid_tool` 规则会直接拦截该工具;`restrict_daily_fen`
   规则会基于用户滚动 24 小时的累计,使用
   `SELECT ... FOR UPDATE` 锁住账户行,重新评估 Red/Yellow
   分界线——这样两个并发请求无法同时通过。

`build_plan` 是唯一调用这些的地方。智能体路由没有任何方式
绕过 `decide_node`。

## 1.3 工具等级表(当前)

| 规范化工具名 | 默认等级 | 备注 |
|---|---|---|
| `accounts.list` / `accounts.balance_summary` / `accounts.balance_check` | Green | 只读。 |
| `transactions.recent`、`spending.report`、`payees.list`、`cards.list`、`subscriptions.list`、`wealth.summary`、`user.profile`、`forecast.simulate` | Green | 只读。 |
| `budgets.list` | Green | 只读。 |
| `reminders.create` | Green | 仅产生通知。 |
| `subscriptions.cancel`、`subscriptions.remind` | Yellow | 可撤销;商家侧动作。 |
| `goals.create_reservation` | Yellow | 资金被持有,未移动。 |
| `budgets.set` | Yellow | 软上限;不移动资金。 |
| `cards.issue`、`cards.action` (freeze / unfreeze / update_limits) | Yellow | 可撤销。 |
| `wealth.subscribe` | Yellow | 须通过 `decide_wealth_subscribe` 的适合性检查。 |
| `wealth.redeem` | Yellow | 须通过持仓状态检查。 |
| `goals.consume_reservation`、`goals.release_reservation` | Yellow | 预留金的结算。 |
| `budgets.reserve` | **Red** | 移动 `available → reserved`。"妻子生日,预留 ¥1000" 这条跨场景流落在这里。 |
| `transfer.create_pending` | **Yellow 或 Red** | `decide_transfer` 重新评估。满足以下任一条件升 Red: 金额超过单笔阈值、收款方未验证、当日累计已达上限、近期多次安全事件、或被 `forbid_tool` / `restrict_daily_fen` 规则命中。 |
| `split_bill.create` | Yellow | 幂等且可完全撤销。 |
| `connectors.merchant.order` | Yellow | 受每商户金额上限约束。 |
| `fraud.score`、`redteam.test`、`voice.tts` | Green | 诊断 / 输出。 |

## 1.4 运行时 Yellow 意味着什么

Yellow 节点**不是**静默执行。智能体路由:

1. 返回一个 `pending_action`,包含 `risk_tier: "Yellow"`、
   `expires_at`,以及一个不可变的 `parameter_hash`
   (规范化 JSON 的 SHA-256)。
2. UI 渲染 `ConfirmationCard`,带清晰的 Yellow 徽标、动作的
   一句话摘要,以及 "确认" / "取消" 一对按钮。
3. 确认时后端重新校验**精确的**参数哈希。若用户(或攻击者)
   在提案与确认之间篡改了 `pending_actions`,确认会返回
   `parameter_hash_mismatch` 且动作不会执行。
4. 动作恰好执行一次。重复的 `confirm` 是 no-op,返回缓存结果。

## 1.5 运行时 Red 意味着什么

Red 节点在确认与执行之间多一步:

1. 重新校验 `parameter_hash` 后,路由调用
   `actions::verify_action`,检查 `MFA_DEMO_CODE`(本演示中) —
   三次失败会把用户标记为 `safety_lock_state = "locked"`。
2. 然后 `actions::confirm_action` 在一个事务里执行工具并
   写审计日志。
3. 若启用了 `forbid_tool` 或 `restrict_daily_fen` 规则,
   确认路径返回 `forbidden` 或 `daily_limit_exceeded`,
   并向用户展示解释,而不是通用失败。

中途失败的 Red 动作会尽可能补偿。例如,一个 MFA 通过后
收款方查找失败的 `transfer.create_pending` 会释放其预留金。
每个 `PlannedNode` 的 `compensation` 字段是规范化的释放工具。

## 1.6 会话、角色、CSRF

| 关注点 | 在哪里强制 |
|---|---|
| 身份认证 | `routes::auth::login` 颁发一个 session cookie (`HttpOnly`、`SameSite=Strict`)。Cookie 是唯一的认证凭据。 |
| 授权 | 每个受保护路由解析 `AuthUser` 提取器;`role == "admin"` 是 `/api/v1/admin/*` 所必需。前端不做任何角色检查。 |
| CSRF | CSRF token 绑定到会话,每个状态变更请求都要求 `X-CSRF-Token`。前端从 `/api/v1/auth/me` 读取它,axios/fetch 封装自动附加。 |
| 安全锁 | `safety::assert_unlocked(user)` 在每个变更路由的开头运行。时间窗内三次 MFA 失败将 `safety_lock_state = "locked"`。 |
| 每日限额 | `restrict_daily_fen` 规则在 `planner::build_plan` 内**以及** 确认时再次评估,且账户行被锁。 |

**下一页:** [§2 — 审计与可观测性 →](./02-audit.md)
