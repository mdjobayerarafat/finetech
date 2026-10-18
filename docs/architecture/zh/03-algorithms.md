---
id: arch-algorithms-zh
title: "3 · 核心算法 (中文)"
sidebar_label: "3 · 核心算法 (中文)"
sidebar_position: 4
---

# 3. 核心算法

> English: [03-algorithms.md](../03-algorithms.md)

FinTech Pioneers 的价值集中在 **七个**具体算法上。本页按统一格式
记录每一个:**意图**、**伪代码**、**源码索引**、**复杂度 /
保证**、**可测试点**。

> **所有算法共享的不可协商规则**: *代码计算,模型解释。*LLM
> 只为后端已经算好的数字配一段叙述。代码库中没有任何路径把
> LLM 的答案直接当成财务数字。

---

## 3.1 策略分级 — `policy::decide_*`

**意图.** 给定一个已校验的工具调用,判定它是 `Green`、
`Yellow` 还是 `Red`,并产出一组人类可读的 flags。决定在
**同一工作流内是单调的**:可以升级,不能降级。

**源码.** `backendrust/src/policy.rs`。逐工具决策函数:
`decide_transfer`、`decide_card_action`、
`decide_wealth_subscribe`、`decide_wealth_redeem`、
`decide_subscription_cancel`、`decide_goal_reservation`、
`decide_budget_set`、`decide_budget_reserve`。

**算法 — `decide_transfer`。**

```
function decide_transfer(user, account, payee, amount_fen, memo):
    if amount_fen <= 0:
        return Decision(RED, "non_positive_amount", flags=[])

    if detect_injection(memo) ≠ ∅:
        return Decision(RED, "injection_pattern",
                        flags=["injection:<phrase>"])

    flags = check_payee_escalation(payee)        # unverified, new
    flags += check_amount_unusual(payee, amount_fen)  # > 3× median

    if check_daily_limit(account, amount_fen) is RED:
        return Decision(RED, "daily_limit_exceeded",
                        escalate_from=YELLOW, flags=flags)

    assessment = fraud::score_transfer(user, payee, amount_fen, memo)
    flags += map(assessment.flags, f → "fraud:" + f.code)
    if any_hard_signal(assessment.flags):       # urgency / flagged / weight ≥ 5
        return Decision(RED, "scam_guard_signal",
                        escalate_from=YELLOW, flags=flags)

    if flags ≠ ∅:
        return Decision(RED, flags.join(","),
                        escalate_from=YELLOW, flags=flags)

    return Decision(YELLOW, "within_daily_limit", flags=[])
```

**保证.**

- **同一工作流内单调** — 不存在 `tier: downgraded` 路径。
  升级是单向的。
- **无整数/浮点漂移** — `amount_fen` 是 `i64`;阈值是
  `config::Settings` 中的常量。
- **memo 先被清洗** — 注入检查发生在 LLM 看到该值之前,
  且 LLM 永远不会把用户提供的自由文本当作指令。

**可测性.** `backendrust/src/policy.rs` 在
`policy::tests` 块(编译期 `#[cfg(test)]`)中进行了详尽
单元测试。覆盖范围:零金额、负金额、日上限突破、收款人
升级 flag、memo 注入短语、反诈硬信号。

---

## 3.2 反诈卫士 — `fraud::score_transfer`

**意图.** 产出一组**信号**(不是分级),让策略引擎和聊天 UI
展示"为什么银行觉得这可能是一笔诈骗?"。LLM 被请求用文字
描述这些信号;分数本身由 Rust 计算。

**源码.** `backendrust/src/fraud.rs`。关键函数:
`score_transfer`、`narrate_scam_flags`、`detect_urgency`、
`detect_flagged_merchant`、`is_round_number`、
`historical_median_fen`、`average_monthly_expense_fen`、
`available_balance_fen`。

**算法.**

```
function score_transfer(user, payee, amount_fen, memo):
    flags = []

    if payee.first_used_at is None:
        flags += [FraudFlag("new_payee", weight=3)]

    if not payee.verified:
        flags += [FraudFlag("unverified_payee", weight=4)]

    median = historical_median_fen(payee.id)        # 过去 3+ 笔
    if median > 0 and amount_fen > 3 * median:
        flags += [FraudFlag("amount_unusual", weight=4)]

    if detect_urgency(memo) ≠ ∅:                     # 双语列表,35+ 短语
        flags += [FraudFlag("urgency_words", weight=5)]

    if detect_flagged_merchant(payee.name) ≠ ∅:      # "stranger", "高回报", …
        flags += [FraudFlag("payee_flagged", weight=5)]

    if is_round_number(amount_fen) and amount_fen ≥ 50_000:
        flags += [FraudFlag("amount_round", weight=1)]

    avg_expense = average_monthly_expense_fen(user.id)   # 90 天借方均值 / 3
    available  = available_balance_fen(user.id)          # 各账户余额之和
    if available - amount_fen < avg_expense:
        flags += [FraudFlag("out_of_pattern", weight=3)]

    score = min(100, sum(weights) * 4)                # 演示用仪表盘
    templated = "This transfer has N signal(s): …" if flags else "No scam indicators."
    return FraudAssessment(flags, score, templated, narrative=None)
```

**硬信号规则.** 在以下任一情况下,转账升级到 **Red**:

- 存在 `urgency_words`,或
- 存在 `payee_flagged`,或
- 任意 flag 的 `weight ≥ 5`。

这条规则由读取评估结果的策略引擎强制,而不是反诈卫士自身 —
`score_transfer` 是给策略引擎的**建议**,它不会返回分级。

**叙述.** `narrate_scam_flags` 仅在 `UNSLOTH_ENABLED=1` 时
调用 LLM。prompt 是信号的简短 JSON;LLM 被请求写 1–2 句。
响应**不**用于任何决策;无论叙述是否存在,分数都保持不变。
若 LLM 失败,使用模板化解释作为回退,UI 永远不为空。

**静态种子列表.** `URGENCY_PHRASES` 是双语的(35+ 英文短语
加上 `马上`、`立刻`、`尽快`、`保密`、`税务局`、`公安局`、
…) 。`FLAGGED_MERCHANT_KEYWORDS` 包含 `stranger`、
`unknown person`、`doubling`、`high return`、`easy money`、
`陌生人`、`高回报`、`投资机会`、`翻倍`。真实银行中这些
来自反诈 feed;这里是静态的并明确标注为 simulation-only。

---

## 3.3 智能转账解析 — `smart_transfers::parse_request`

**意图.** 把一段自由形式的转账请求("转 200 元午饭钱给
李伟,明天,和 Alice 分账")转成 planner 期望的同一份 JSON
形状:`amount_fen`、`recipient_query`、`memo`、`schedule_iso`、
`split_participants`、`needs_clarification`、`questions[]`。

**源码.** `backendrust/src/smart_transfers.rs`。入口:
`parse_request`、`suggest_payees`、`resolve`。

**算法 — LLM 路径(当 `UNSLOTH_ENABLED=1`)。**

```
function llm_parse(user, text):
    accounts = list(user's accounts)              # id, display_name, available_balance_fen
    prompt = {
        system: "… return STRICT JSON: {amount_fen, recipient_query,
                  memo, schedule_iso, split_participants,
                  needs_clarification, questions[]}",
        accounts: accounts,
        text: text,
    }
    response = openai_chat(model=UNSLOTH_MODEL, temperature=0.2,
                            response_format={"type":"json_object"}, …)
    parsed = json(response.content)
    resolved = resolve(user, parsed.recipient_query)   # 始终由服务端运行
    return parsed + {
        resolved_payee: resolved.payee,
        resolve_status:  resolved.status,         # resolved | ambiguous | none
        candidates:      resolved.candidates,
    }
```

**算法 — stub 路径(离线 / LLM 不可用)。**

```
function stub_parse(text):
    amount_fen = regex_amount(text)               # (\d+)([.,]\d{1,2})? (元|块|RMB|¥|￥|yuan)?
    recipient_query = first_token_after(["pay ","to ","send to ","给","转给"])
                       OR 最长的 CJK token
                       OR 第一个大写 ASCII token
    schedule_iso = if "tomorrow" / "明天" / 等  → now + 1d
                   if "next week" / "下周"      → now + 7d
                   if "next month" / "下月"     → now + 30d
                   else None
    wants_split  = "split" ∈ lower OR "均摊" ∈ text OR "分摊" ∈ text
    memo         = " for " / "memo:" / "因为" / "——" 之后的子串,裁到 200 字符
    return { amount_fen, recipient_query, memo, schedule_iso,
             split_participants, needs_clarification, questions[] }
```

**解析步骤**(两条路径共享)。

```
function resolve(user, query):
    suggestions = suggest_payees(user, query, limit=5)   # DB 查询 + 打分
    if suggestions 为空:
        return ResolveResult(status="none",
            note="No saved payee matches. Add a beneficial account first.")
    if len == 1 or top.confidence ≥ 0.95:
        return ResolveResult(status="resolved", payee=suggestions[0].payee)
    return ResolveResult(status="ambiguous", candidates=suggestions)
```

`PayeeSuggestion` 携带 `match_kind`(`exact_name`、
`phone_tail`、`prefix_name`、`contains_name`)与 `confidence`
(1.0, 0.98, 0.85, 0.6)。

**安全审查.** `security_review` 是一个单独的工具,在**草稿**
上调用。它跑 `fraud::score_transfer` + 注入与紧急词检测,
然后返回 `SecurityReview` 结构,字段为 `recommended_tier`、
`fraud_score`、`flags`、`injection_phrase`、`urgency_detected`、
`summary`(LLM 或模板)与 `schedule_note`。**`recommended_tier`
仍然只是建议** — 真正的分级在用户确认时由
`policy::decide_transfer` 决定。

---

## 3.4 "我负担得起吗?" — `forecast::run`

**意图.** 给定一个情景(`months` 个月后花 `amount_fen`),
用 90 天滚动均值与已排程的提醒,把未来 `months` 个月的余额
投影出来,并返回 verdict(`yes` / `tight` / `no`)与一段叙述。

**源码.** `backendrust/src/forecast.rs`。入口:
`run(pool, settings, user, amount_fen, spend_month_index, months)`。

**算法.**

```
function run(user, amount_fen, spend_month_index, months):
    months = clamp(months, 1, 24)
    spend_month_index = clamp(spend_month_index, 0, months - 1)

    starting      = available_balance_fen(user.id)
    avg_income    = monthly_cashflow_fen(user.id).income   # 90 天贷方和 / 3
    avg_expense   = monthly_cashflow_fen(user.id).expense  # 90 天借方和 / 3
    monthly_subs  = monthly_subscriptions_fen(user.id)     # 对 active 求 Σ(amt × 30 / interval_days)
    held          = held_reservations_fen(user.id)         # state='held' 的 reservations
    budgets       = active_budgets(user.id)                # 每类上限 / 30 天支出 / 是否超限

    upcoming = upcoming_bills_in_window(user.id, from=first_of_next_month, to=+months)

    effective_starting = starting - held                  # 已承诺资金会压低基线
    series = build_series(effective_starting, avg_income, avg_expense,
                          monthly_subs, upcoming, months,
                          starting_month=first_of_next_month,
                          scenario_amount_fen=amount_fen,
                          spend_month_index=spend_month_index)

    end_balance  = series[-1].projected_balance_fen
    baseline_end = series[-1].baseline_fen
    v = verdict(effective_starting, end_balance, amount_fen)
    narrative = narrate(...) or templated(v, end_balance, baseline_end, months)
    return ForecastOut(..., v, narrative, held, budgets, total_budget_limit=Σ limit_fen)
```

**`build_series` 是逐月循环。** 对每个月 `i`:

```
bal += (avg_income - avg_expense) - sum(本月到期的 bill)
baseline_bal += (avg_income - avg_expense) - sum(本月到期的 bill)
if i == spend_month_index and amount_fen > 0:
    bal -= amount_fen
push SeriesPoint(month_index=i, label="Jan 2026", …)
```

> **注意.** 订阅**不**被重复扣减。它们已经包含在 `avg_expense`
> 中(出现在交易历史里)。预测会展示 `monthly_subs` 字段,
> 但不会双扣。

**`verdict`.**

```
if end_balance < 0:                          return "no"
elif end_balance < max(amount_fen/2, 50000): return "tight"
else:                                        return "yes"
```

**叙述.** 仅当 `UNSLOTH_ENABLED=1` 时调用 LLM。系统 prompt
明确说:"数字必须完全一致;不要捏造。"

---

## 3.5 收据扫描 — `receipt::scan`

**意图.** 上传一张收据/发票照片,产出一份**草稿**转账
(`payee_hint`、`amount_fen`、`due_at`、`proposed_arguments`),
用户单击确认即可。

**源码.** `backendrust/src/receipt.rs`。入口:
`scan(state, auth, rid, filename, declared_hint, declared_amount, bytes_len)`。

**算法.**

```
function scan(filename, declared_hint, declared_amount, bytes_len):
    draft = if UNSLOTH_ENABLED:
                llm_extract(...) ?? stub_extract(...)
            else:
                stub_extract(...)
    audit::append_event(action="receipt.scan",
                        parameters_masked={filename[:64], bytes, source, confidence},
                        result="ok")
    return draft
```

**Stub 确定性.** `stub_extract` 使用
`(filename.len() XOR declared_hint.len()) % len(DEMO_RECEIPTS)`
从 5 个确定性演示收据中选一个(Sun Cafe 拿铁、Acme Books 发票、
Cell Co. 通信费、CityPower 电费、University 学费)。同一输入
同一结果 — 对演示很重要。

**审计卫生.** 仅记录 `filename[:64]`、`bytes`、`source` 与
`confidence`。抽出的 payee 与金额**不**进 `parameters_masked`
(用户可以从 draft 中再次看到)。

**硬保证.** 扫描的收据**永远**是草稿。它从不直接执行。
发起一笔转账的唯一方式是用户在确认后走 `transfer.create_pending`。

---

## 3.6 规划器 DAG — `planner::build_plan` + `decide_node`

**意图.** 从一份已校验的 `Proposal` 构建一个由一个或多个
工具节点组成的工作流。规划器是**唯一**调用 `policy::decide_*`、
`tool_spec::validate_arguments` 与 agent 规则引擎的代码。

**源码.** `backendrust/src/planner.rs`。静态工具注册表放在
`tool_spec.rs::SPECS`(`Lazy` 初始化的 `Vec<ToolSpec>`);每条
记录有 `canonical_name`、`mutating`、`default_tier`、
`arguments_schema`,以及必需/可选的 `SlotSpec` 列表。

**算法.**

```
function build_plan(user, proposal):
    plan = []
    for action in proposal.proposed_actions:
        tool = canonicalize_tool_name(action.tool)
        spec = tool_spec::spec_for(tool)
        if not spec: continue                          # 未知工具 → 静默跳过
        if tool is in forbidden_tools():               # 管理员规则
            return PlanError(Forbidden, audit("agent_rule.match"))
        validate_arguments(spec, action.arguments)      # schema + slot 检查
        slot_bindings = 从对话记忆中收集 slot_bindings
        for slot in spec.required_slots:
            if slot_bindings[slot.name] is empty:
                if slot.inferable: 尝试 retriever / wallet 视图
                else:               return PlanError(Clarification, slot.question)

        node = decide_node(user, spec, action.arguments)
        plan.append(node)
    return plan

function decide_node(user, spec, args):
    # 1) 工具专属决策函数(若有)
    decision = call decide_<tool>(args) or Decision(default_tier=spec.default_tier)

    # 2) 管理员规则
    decision = apply_agent_rules(decision, user, args)  # 可升级,不可降级

    # 3) parameter_hash + (对 Yellow/Red) pending_action 行
    if decision.tier in (YELLOW, RED):
        parameter_hash = canonical::parameter_hash(args)
        pending_id = insert pending_actions(
            tool=spec.canonical_name, arguments=args, risk_tier=tier,
            policy_reason=decision.reason, parameter_hash, expires_at=now+5m
        )
        return PlannedNode(risk_tier=tier, pending_action_id=pending_id, parameter_hash)

    # 4) Green → 直接执行
    return PlannedNode(risk_tier=GREEN)                   # 没有 pending 行
```

**补偿.** 每个节点声明一个 `compensation` 字符串(例如
`"reverse_transfer"`)。若下游节点失败,执行器反向走 DAG
并对每个已完成的节点运行补偿函数。这让工作流具备"全有或全无"
的特性,而不需要用户再次确认。

**幂等性.** 一个工具的 `idempotency_key` 为
`workflow_id + node_id + parameter_hash`。重放是 no-op —
底层 SQL 使用 `INSERT … ON CONFLICT (idempotency_key) DO NOTHING`。

---

## 3.7 订阅检测 — 启发式(仅 admin 页)

**意图.** 在用户交易历史里把周期性扣款找出来,这样 agent
可以提示"要不要取消 X?"。检测在服务端路由
(`routes/subscriptions.rs::detect`)里跑,不在聊天路径上实时
运行。

**算法 — 滑动窗口。**

```
function detect(user, lookback_days=180, min_occurrences=3):
    debits = SELECT merchant, SUM(-amount_fen), COUNT(*), MIN(occurred_at), MAX(occurred_at)
             FROM transactions
             WHERE account.user_id = user.id
               AND amount_fen < 0
               AND occurred_at >= NOW() - INTERVAL lookback_days
             GROUP BY merchant
             HAVING COUNT(*) >= min_occurrences
    for row in debits:
        interval_days = (MAX - MIN) / (COUNT - 1)
        if interval_days in 25..35:  bucket = "monthly"
        elif interval_days in 6..8:  bucket = "weekly"
        elif interval_days in 12..16: bucket = "biweekly"
        elif interval_days in 85..95: bucket = "quarterly"
        elif interval_days in 350..380: bucket = "annual"
        else: continue
        upsert subscriptions(user, merchant=row.merchant,
                              estimated_amount_fen=row.SUM/row.COUNT,
                              interval_days=interval_days,
                              next_expected_at=row.MAX + interval_days,
                              evidence={...})
```

**启发式的局限(代码注释中已说明)。**

- 种子数据集的间隔不规整 — 种子把明显的标记为 `active`,
  其余的留空。
- 年度订阅用 180 天历史很难抓到。
- 一个商户一笔大额、其余都是小额的,会按小额的来分类。
- 真实环境会换成专门的周期扣款识别流水线(Stripe Sigma、
  Plaid Recurring、内部实现)。模拟代码的注释里写得很清楚。

---

## 3.8 工具循环终止

LLM 在**每个聊天轮次里只被调用一次**(`llm::propose`)。
规划器把它的 `proposed_actions[]` 变成一张 DAG。生产中
**没有** "调 LLM → 看工具结果 → 再调 LLM" 的迭代循环。
原因:每次 LLM 调用都增加延迟和提示注入的风险。stub proposer
基于正则,确定性,所以循环又快又安全。

单轮结构让 `parameters_masked` 足够:LLM 永远看不到工具后状态。
用户的下一轮是另一次 `propose` 调用,带上一份由 `retriever.rs`
从审计日志与对话记忆里重建的 `state_summary`。

---

## 3.9 算法汇总表

| # | 算法 | 输入 | 输出 | 源码 | 用 LLM? |
|---|---|---|---|---|---|
| 3.1 | 策略分级 | (user, account, payee, amount, memo) | `Decision`(分级 + flags) | `policy.rs::decide_*` | 否 |
| 3.2 | 反诈卫士 | (user, payee, amount, memo) | `FraudAssessment`(flags + score + 叙述) | `fraud.rs` | 仅叙述 |
| 3.3 | 智能转账解析 | (user, 自由文本) | `{amount_fen, recipient, memo, schedule, …}` | `smart_transfers.rs` | 可选 |
| 3.4 | 预测 | (user, amount, month, months) | `ForecastOut`(序列 + verdict + 叙述) | `forecast.rs` | 仅叙述 |
| 3.5 | 收据扫描 | (文件元数据, hints) | `ReceiptDraft` | `receipt.rs` | 可选 |
| 3.6 | 规划器 DAG | `Proposal` | `PlannedNode[]`,带分级 + pending 行 | `planner.rs` + `tool_spec.rs` | 否 |
| 3.7 | 订阅检测 | (user, lookback) | upsert subscriptions | `routes/subscriptions.rs` | 否 |
| 3.8 | LLM 工具循环 | 聊天轮次 | (单次 propose) | `llm.rs` | 是(有 stub) |
