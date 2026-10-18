---
id: arch-data-model-zh
title: "2 · 数据模型 (中文)"
sidebar_label: "2 · 数据模型 (中文)"
sidebar_position: 3
---

# 2. 数据模型

> English: [02-data-model.md](../02-data-model.md)

Schema 是普通 PostgreSQL,写在 `backendrust/migrations/` 下的
6 个迁移文件中。迁移是**幂等**的,按文件名字母序执行。

## 2.1 实体关系图

```mermaid
erDiagram
    USERS ||--o{ SESSIONS : "拥有"
    USERS ||--o{ ACCOUNTS : "拥有"
    USERS ||--o{ PAYEES : "拥有"
    USERS ||--o{ CARDS : "持有"
    USERS ||--o{ SAVINGS_GOALS : "追踪"
    USERS ||--o{ SUBSCRIPTIONS : "追踪"
    USERS ||--o{ REMINDERS : "追踪"
    USERS ||--o{ WEALTH_HOLDINGS : "拥有"
    USERS ||--o{ BUDGETS : "拥有"
    USERS ||--o{ BUDGET_RESERVATIONS : "拥有"
    USERS ||--o{ CONVERSATIONS : "拥有"
    USERS ||--o{ PENDING_ACTIONS : "等待"
    USERS ||--o{ SAFETY_EVENTS : "产生"
    USERS ||--o{ AGENT_RULES : "created_by (admin)"
    USERS ||--o{ AUDIT_EVENTS : "actor"
    USERS ||--o{ WORKFLOWS : "拥有"

    ACCOUNTS ||--o{ TRANSACTIONS : "记账"
    ACCOUNTS ||--o{ TRANSFERS : "源账户"
    ACCOUNTS ||--o{ RESERVATIONS : "源账户"
    ACCOUNTS ||--o{ BUDGET_RESERVATIONS : "源账户"
    ACCOUNTS ||--o{ SPLIT_BILLS : "源账户"

    PAYEES ||--o{ TRANSFERS : "收款"
    PAYEES ||--o{ SPLIT_BILL_PARTS : "参与人"

    TRANSFERS ||--o{ SPLIT_BILL_PARTS : "已支付"

    SAVINGS_GOALS ||--o{ RESERVATIONS : "预占"
    SAVINGS_GOALS ||--o{ BUDGET_RESERVATIONS : "预占"

    SUBSCRIPTIONS ||--o{ REMINDERS : "关联"

    WEALTH_PRODUCTS ||--o{ WEALTH_HOLDINGS : "申购"

    BUDGETS ||--o{ BUDGET_RESERVATIONS : "锚定"

    CONVERSATIONS ||--o{ CONVERSATION_MESSAGES : "包含"
    USERS ||--o{ CONVERSATION_MESSAGES : "作者"

    WORKFLOWS ||--o{ WORKFLOW_NODES : "执行"
    WORKFLOW_NODES ||--o{ PENDING_ACTIONS : "可等待"

    SPLIT_BILLS ||--o{ SPLIT_BILL_PARTS : "拆分"

    USERS {
      TEXT id PK
      TEXT username UK
      TEXT password_hash
      TEXT display_name
      TEXT preferred_language
      TEXT role
      TEXT safety_lock_state
      TEXT safety_lock_reason
      INT  failed_mfa_count
      TEXT risk_tolerance
      TEXT horizon
    }
    SESSIONS {
      TEXT id PK
      TEXT user_id FK
      TEXT csrf_token
      TIMESTAMPTZ expires_at
    }
    ACCOUNTS {
      TEXT id PK
      TEXT user_id FK
      TEXT display_name
      TEXT masked_number
      TEXT currency
      BIGINT available_balance_fen
      BIGINT reserved_balance_fen
    }
    TRANSACTIONS {
      TEXT id PK
      TEXT account_id FK
      BIGINT amount_fen
      TEXT direction
      TEXT category
      TEXT merchant
      TEXT memo
      TIMESTAMPTZ occurred_at
      TEXT status
    }
    PAYEES {
      TEXT id PK
      TEXT owner_user_id FK
      TEXT display_name
      TEXT phone_tail
      TEXT mock_destination
      BOOLEAN verified
      TIMESTAMPTZ first_used_at
    }
    TRANSFERS {
      TEXT id PK
      TEXT source_account_id FK
      TEXT payee_id FK
      BIGINT amount_fen
      TEXT memo
      TIMESTAMPTZ schedule
      TEXT idempotency_key UK
      TEXT status
      TIMESTAMPTZ posted_at
    }
    SUBSCRIPTIONS {
      TEXT id PK
      TEXT user_id FK
      TEXT merchant
      BIGINT estimated_amount_fen
      INT  interval_days
      TIMESTAMPTZ next_expected_at
      TIMESTAMPTZ last_charge_at
      JSONB evidence
      TEXT cancellation_status
    }
    REMINDERS {
      TEXT id PK
      TEXT user_id FK
      TEXT kind
      TEXT subject
      TIMESTAMPTZ due_at
      TEXT status
      TEXT linked_subscription_id FK
    }
    SAVINGS_GOALS {
      TEXT id PK
      TEXT user_id FK
      TEXT name
      BIGINT target_fen
    }
    RESERVATIONS {
      TEXT id PK
      TEXT goal_id FK
      BIGINT amount_fen
      TEXT state
      TEXT source_account_id FK
    }
    CARDS {
      TEXT id PK
      TEXT user_id FK
      TEXT display_name
      TEXT masked_number
      TEXT status
      BIGINT single_tx_limit_fen
      BIGINT monthly_limit_fen
    }
    WEALTH_PRODUCTS {
      TEXT id PK
      TEXT name
      TEXT category
      TEXT risk_level
      DOUBLE PRECISION expected_annual_return_pct
      BIGINT min_subscription_fen
      TEXT suitability_required
    }
    WEALTH_HOLDINGS {
      TEXT id PK
      TEXT user_id FK
      TEXT product_id FK
      BIGINT principal_fen
      TIMESTAMPTZ started_at
    }
    BUDGETS {
      TEXT id PK
      TEXT user_id FK
      TEXT category
      BIGINT limit_fen
      TIMESTAMPTZ period_start
      TIMESTAMPTZ period_end
      TEXT account_id FK
      TEXT note
    }
    BUDGET_RESERVATIONS {
      TEXT id PK
      TEXT user_id FK
      TEXT budget_id FK
      TEXT goal_id FK
      TEXT label
      BIGINT amount_fen
      TIMESTAMPTZ due_at
      TEXT source_account_id FK
      TEXT state
    }
    PENDING_ACTIONS {
      TEXT id PK
      TEXT user_id FK
      TEXT workflow_id FK
      TEXT node_id FK
      TEXT tool
      JSONB arguments
      TEXT risk_tier
      TEXT policy_reason
      TEXT parameter_hash
      TIMESTAMPTZ expires_at
      TEXT status
    }
    SAFETY_EVENTS {
      TEXT id PK
      TEXT user_id FK
      TEXT kind
      TEXT details
      TIMESTAMPTZ created_at
    }
    AGENT_RULES {
      TEXT id PK
      TEXT kind
      TEXT name
      TEXT pattern
      TEXT severity
      TEXT applies_to
      BOOLEAN enabled
      TEXT created_by FK
    }
    AUDIT_EVENTS {
      TEXT id PK
      TIMESTAMPTZ created_at
      TEXT actor_user_id FK
      TEXT session_id
      TEXT request_id
      TEXT workflow_id
      TEXT node_id
      TEXT action
      TEXT tool
      JSONB parameters_masked
      TEXT risk_tier
      TEXT policy_reason
      TEXT confirmation_status
      TEXT mfa_status
      TEXT result
      TEXT error_code
      TEXT prev_hash
      TEXT hash
    }
    WORKFLOWS {
      TEXT id PK
      TEXT user_id FK
      TEXT status
      JSONB plan
    }
    WORKFLOW_NODES {
      TEXT id PK
      TEXT workflow_id FK
      TEXT tool
      JSONB arguments
      TEXT risk_tier
      TEXT status
      TEXT idempotency_key
    }
    CONVERSATIONS {
      TEXT id PK
      TEXT user_id FK
      TEXT title
    }
    CONVERSATION_MESSAGES {
      TEXT id PK
      TEXT conversation_id FK
      TEXT user_id FK
      TEXT role
      TEXT content
      JSONB meta
    }
    SPLIT_BILLS {
      TEXT id PK
      TEXT owner_user_id FK
      TEXT title
      TEXT memo
      BIGINT total_fen
      TEXT currency
      TEXT source_account_id FK
      TEXT status
      TIMESTAMPTZ due_at
    }
    SPLIT_BILL_PARTS {
      TEXT id PK
      TEXT split_bill_id FK
      TEXT payee_id FK
      TEXT participant_name
      TEXT participant_phone_tail
      BIGINT share_fen
      TEXT status
      TIMESTAMPTZ paid_at
      TEXT paid_transfer_id FK
    }
```

## 2.2 迁移时间线

| 文件 | 引入 |
|---|---|
| `20251020000000_init_pg.sql` | `users`, `sessions`, `accounts`, `transactions`, `payees`, `transfers`, `subscriptions`, `reminders`, `savings_goals`, `reservations`, `cards`, `wealth_products`, `wealth_holdings` |
| `20251115000000_agent_v2.sql` | `workflows`, `workflow_nodes`, `pending_actions`, `safety_events`, `audit_events`(带哈希链) |
| `20251118000000_chat_memory.sql` | `conversations`, `conversation_messages` |
| `20251120000000_smart_transfers.sql` | `split_bills`, `split_bill_parts`, 状态重算触发器 |
| `20251121000000_admin_panel.sql` | `agent_rules`, `audit_events`/`safety_events`/`conversations`/`workflows` 的管理员监控索引 |
| `20251122000000_budgets.sql` | `budgets`, `budget_reservations` |

## 2.3 金额一律用整数分

**每一个**金额列都是 `BIGINT *_fen`(1/100 元)。账本数学中
**不存在 `f64`**。这一点在 schema(例如 `transfers`、
`budgets`、`budget_reservations`、`split_bills`、
`split_bill_parts` 上的 `CHECK (amount_fen > 0)`)和应用层
("规划器绝不输出 `amount_fen` 的非整数值")两侧同时强制。

| 字段 | 类型 | 写入位置 | 如何归一到分 |
|---|---|---|---|
| `accounts.available_balance_fen` | BIGINT | `t_transfer_create_pending`、`t_card_*`、seed | 始终为整数 |
| `accounts.reserved_balance_fen` | BIGINT | `t_goal_reservation`(Yellow)→ `t_budget_reserve`(Red) | 始终为整数 |
| `transfers.amount_fen` | BIGINT | planner | `money::coerce_amount_fen`,从 CNY 字符串归一 |
| `transactions.amount_fen` | BIGINT | 工具副作用 | 符号 = `direction` |
| `subscriptions.estimated_amount_fen` | BIGINT | `detect_subscriptions` 启发式 | LLM 抽取或规则抽取 |
| `budgets.limit_fen` | BIGINT | `t_budget_set` | Yellow 级,仅整数 |
| `budget_reservations.amount_fen` | BIGINT | `t_budget_reserve` | Red 级,仅整数 |
| `wealth_holdings.principal_fen` | BIGINT | `t_wealth_subscribe` | 适当性检查后 Yellow 级 |
| `split_bills.total_fen` / `share_fen` | BIGINT | `smart_transfers::create_split_bill` | 应用层校验 sum 不变量 |

> **展示规则.** 前端用 `Intl.NumberFormat` 配合当前 locale 的
> 货币样式格式化。**不**做服务端 float 转换;唯一一次到
> 小数的转换发生在 UI。

## 2.4 审计哈希链

`audit_events` 是**追加式**的,带 SHA-256 链。每一行都记
`prev_hash` 与 `hash`,其中:

```
hash = sha256(
    canonical_json({
        actor_user_id, session_id, request_id,
        workflow_id, node_id, action, tool,
        parameters_masked, risk_tier, policy_reason,
        confirmation_status, mfa_status, result, error_code,
        created_at, prev_hash,
    })
)
```

创世哈希为 `00…00`(32 字节)。`audit::last_hash` 总是读取最新
一行。`audit::verify_chain` 按时间顺序遍历全表并重算 digest,
O(n)。任何篡改 — 包括事后改 `parameters_masked` — 都会在
下一次 verify 时打断链。

`parameters_masked` 是阻止链泄露密钥的字段。敏感值在 canonical
JSON 哈希之前会被占位符(如 `***`)替换;原始值**永远不**写入
审计日志。

## 2.5 索引

关键查询路径均建有索引:

- `ix_transactions_account_occurred (account_id, occurred_at)` —
  dashboard 的 12 个月图表查询。
- `ix_audit_events_user_time (actor_user_id, created_at DESC)` —
  admin 审计页。
- `ix_safety_events_user_time (user_id, created_at DESC)` — admin
  安全页。
- `ix_conversation_messages_user_time (user_id, created_at DESC)` —
  聊天历史列表。
- `ix_workflows_user_time (user_id, created_at DESC)` — 工作流列表。
- `ix_agent_rules_enabled (enabled, kind)` — 规则引擎热路径。
- `idx_budgets_user_category (user_id, category)` — 预算上限查询。
- `ix_split_bills_status (status)` — "还欠多少" 过滤器。

## 2.6 并发模型

- 所有状态变更写都在一个 Postgres 事务中完成。
- 余额减少前先 `SELECT … FOR UPDATE` 锁住 `accounts` 行。
  这是**唯一**可以写 `available_balance_fen` 的方式,所以同一
  账户的两个并发转账会串行化,而不会双花。
- 日转账上限(`policy::check_daily_limit`)在插入新行的事务
  内部读并累加 `transfers`,所以并行的审批不会突破上限。
- 工作流节点使用 `workflow_id + node_id + parameter_hash` 作为
  幂等键。retry 是 no-op。
