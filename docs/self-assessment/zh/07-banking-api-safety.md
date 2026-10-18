---
id: banking-api-safety-zh
title: "7 · 银行与 API 安全"
sidebar_label: "7 · 银行 / API 安全"
sidebar_position: 8
---

# 7. 银行与 API 安全

> English: [07-banking-api-safety.md (EN)](../07-banking-api-safety.md)

[§3](./03-llm-safety.md) 覆盖了大模型 / 智能体攻击。本页是
**银行与 HTTP API** 对应的一面——OWASP API 与金融服务清单中
十二类具名攻击,各自如何命中 FinTech Pioneers,以及被哪道具体防线
挡下。同样的诚实规则:每行 *防线* 指向代码,每个缺口在
[§4](./04-known-risks.md) 中具名。

## 7.1 认证与会话

| # | 攻击 | 如何命中 FinTech Pioneers | 防线 | 位置 |
|---|---|---|---|---|
| 1 | **认证失效——弱 JWT、不过期、令牌复用** | demo 级别的认证如果发 `alg: none` 或永不过期的令牌,会话 cookie 一旦泄露(XSS、日志泄露、cookie 错配),攻击者就拿到一个永久会话。 | 我们**不用 JWT**。会话是 `sessions` 表里的服务端行,带 `expires_at`(默认 12 小时,由环境变量 `SESSION_TTL_SECONDS` 驱动)。Cookie 是不透明串(`Cookie::http_only(true)`、`SameSite::Lax`、生产环境 `Secure`)。CSRF token 是与 cookie 一起生成的每会话随机串。没有"忘记关 alg: none"的脚枪——根本就没签名密钥。 | `sessions.rs::create_session`、`sessions.rs::cookie_attrs` |
| 2 | **权限变更后旧会话仍有效** | 在角色 / 锁定 / MFA 失败状态变更之前签发的会话,保留旧权限。 | **缺口(R-7)**。当前会话在 `role`、`safety_lock_state`、`password_hash` 变更时**不会**重新签发。加固路径在 `04-known-risks.md` (R-7)。Demo 期间唯一状态会变化的用户是 `locked_vip`,而该用户在 seed 时就已经锁定,所以陈旧会话窗口是有界的。 | `safety.rs`、§4 R-7 |
| 3 | **暴力枚举**——登录、OTP、收款方查询 | 脚本化攻击者试 100 个最常见密码,或遍历手机尾号 0000…9999 来枚举收款方。 | (a) **MFA 失败**按*用户*维度限速:`MAX_FAILED_MFA=3` 次连续错码会触发 `safety_lock_state = locked` 并写一条审计。(b) **登录**没有按 IP 限速(R-6 列为加固路径);demo 依赖小用户基数 + Argon2id 密码哈希。(c) **收款方查询**对未知手机尾号与未知 ID 返回同一通用 `not_found`——没有枚举预言机。 | `safety.rs::record_failed_mfa`、`errors.rs::not_found`、§4 R-6 |

## 7.2 授权

| # | 攻击 | 如何命中 FinTech Pioneers | 防线 | 位置 |
|---|---|---|---|---|
| 4 | **IDOR / BOLA**——把 `account_id` 改成别人的来读数据 | API 漏洞里最大的一类。任何接受用户控制 id 并返回数据的端点都是候选。 | **所有权在工具层逐行复核**,不在登录时复核。每个接受 `id` 的写工具在做任何事之前都会做 `if row.user_id != ctx.user.id { return ToolError::NotFound(...) }`。同样的 `user_id` 谓词出现在每条读 SQL(`WHERE a.user_id = $1`)。套件里的跨用户测试断言:用户 A 拿用户 B 的账户 id 得到 404,而不是 200。 | `tools.rs::t_*`、`cross_user.rs` |
| 5 | **确认卡 UI 篡改(UI-redress)**——模型文本说"你将付给 Alice",确认卡说"你将付给 Eve" | 如果 UI 用模型散文渲染确认,模型与动作可能不一致,用户会确认了不同的内容。 | `ConfirmationCard` 由**类型化的 `pending_action`** 渲染,不用任何模型字段。卡片读 `pending_action.tool` 与对应 `plan.nodes[…].parameters`——两者都是服务端校验过、不可变、确认前锁定的。模型叙述只在卡片上方作装饰显示,永远不是真实来源。CSRF token 与 `pending_action_id` 绑定,被偷 / 被重放的卡片无法再次点火。 | `frontend/src/components/confirmation-card`、`routes/actions.rs::confirm_action` |

## 7.3 交易完整性

| # | 攻击 | 如何命中 FinTech Pioneers | 防线 | 位置 |
|---|---|---|---|---|
| 6 | **重放 / 双花**——重提交一笔已确认的转账 | 网络抖动、用户双击、客户端 bug 重试——同一笔转账被结算两次。 | (a) `t_transfer_create_pending` **要求** `idempotency_key`。同 key + 同参数返回已有 `pending_action`;同 key + *不同*参数返回 `409 conflict`。(b) `pending_actions` 有 `confirmation_status` 生命周期(`pending → confirmed → executed`);一旦 `executed`,再次确认返回 `409 already_executed`。(c) `expires_at`(默认 5 分钟)让任何未确认的动作在窗口外无法被重放。 | `tools.rs::t_transfer_create_pending`、`routes/actions.rs::confirm_action` |
| 7 | **竞态条件**——两笔并发转账透支账户 | 两个 `POST /transfers` 同时到达;两者读到同一余额,都通过 `>= amount` 检查,都成功。 | 每个写工具都打开一个 `sqlx` 事务并调用 `lock_account(&mut *tx, account_id)`,后者会发出 `SELECT … FROM accounts WHERE id = $1 AND user_id = $2 FOR UPDATE`。第二笔转账阻塞,直到第一笔 commit,然后读*新*余额并触发可用余额检查。中间任一环节出错工具就 `commit`——不留部分状态。 | `tools.rs::lock_account` |
| 8 | **业务逻辑滥用**——拆大笔转账规避限额(structuring);探测有效收款方 | 攻击者用 9 × ¥999 代替 1 × ¥9,000 绕过单笔上限;或者猛刷 `/payees?phone_tail=…` 来枚举。 | (a) **`restrict_daily_fen` 规则**对单账户*累计*金额设上限(默认 `DAILY_LIMIT_FEN = 1_000_000` 分 = ¥10,000),按滚动窗口。规划器拒绝 `today_fen + attempted_fen > cap` 的节点,而不是只查 `attempted_fen > cap`。解释器同时显示两个数。(b) **单笔 `restrict_amount_fen`** 守上界。(c) **收款方查询**对"未知"和"知道但不是你的"返回同一通用 `not_found`;攻击者无法区分"没有这个收款方"与"不是你的收款方"。(d) **按用户的速度计数器**是加固路径(R-6),代码里还没有。 | `agent_rules.rs`、`policy_explainer.rs`、§4 R-6 |
| 9 | **MFA 疲劳 / 绕过**——对用户狂弹提示直到通过;或通过别的端点绕过 MFA | 被入侵的聊天会话反复提议 Red 等级动作;或 Red 等级工具在没 MFA 的路径下被暴露。 | (a) **`MAX_FAILED_MFA = 3`**——第 3 次错码即锁定账户。(b) **MFA 与动作绑定**:`verify_action` 端点同时要求 `pending_action_id` 与 `mfa_code`;码不能跨动作复用。(c) **MFA 在每条 Red 节点的执行路径服务端强制**——见 `routes/actions.rs::confirm_action`——没有绕过它的 Red 等级执行路径。(d) 大模型不能把动作标为 Green 或 Yellow;tier 只在 `policy.rs` 里决定。(e) **MFA 疲劳(对同一用户多次提示)**是加固路径:未来要加一个独立于失败计数的、按用户的 `mfa_prompt` 速率计数器。 | `safety.rs`、`routes/actions.rs::verify_action`、`policy.rs` |

## 7.4 输入 / 输出处理

| # | 攻击 | 如何命中 FinTech Pioneers | 防线 | 位置 |
|---|---|---|---|---|
| 10 | **SQL 注入** | 用户控制的字段(收款方名、备注、搜索)被拼接到 SQL 字符串。 | (a) **每条查询都通过 `sqlx::query` + `.bind` 参数化**——`sqlx` 宏拒绝编译未类型化的字符串插值。(b) 仓库范围 grep(`! grep -RE 'query!\([^)]*\+' src/`)是回归网;`query!` 宏里任何 `format!` 都会让 CI 失败。(c) `ORDER BY` 等不能接受 bind 参数的 SQL 片段,在少数接受它们的工具里只允许硬编码白名单(例如 `transactions.list` 只能接受固定集合的 order 键)。 | `tools.rs::t_*`、`transactions.rs`、`tests/` 中的 grep 守卫 |
| 11 | **聊天与通知里的 XSS** | 模型输出或商户文本被当作 HTML 渲染。备注里一个 `<img onerror=…>` 就能偷走会话 cookie。 | (a) 聊天 / 小票 / 通知路径**没有任何 `dangerouslySetInnerHTML`**(仓库范围 grep 是测试)。(b) 所有用户与模型提供的文本都按 React 文本渲染,默认即转义。(c) **严格的 CSP 当前没设**(R-8 列为加固路径);demo 依赖 React 转义。(d) 小票将来要做 Markdown 渲染时,必须走 sanitizer;审计日志的 `parameters_masked` 对已存的模型输入已做等价处理。 | `frontend/src/`、§4 R-8、`audit.rs::mask_parameters` |
| 12 | **SSRF / 不安全连接器** | 商户或小票连接器工具去拉用户(或模型)给的 URL。攻击者用 `http://10.0.0.5/…` 探测内网。 | (a) **没有工具接收用户控制的 URL。**小票上传走的是类型化连接器(`connectors.merchant.order`),只接受内部商户 id,在服务端解析。(b) **任何将来的 URL 拉取**都必须经过唯一的 `http_client::safe_get(url)` 包装器——解析 DNS、屏蔽 RFC1918 / link-local / loopback / `*.internal`,且只允许 `https://` 命中维护过的白名单。(c) 现有连接器集合足够小,代码评审清单就够覆盖;那个包装器是加固路径。 | `routes/agent.rs::configure`、§4 R-3 / R-15 |

## 7.5 供应链、密钥、可观测性

| # | 攻击 | 如何命中 FinTech Pioneers | 防线 | 位置 |
|---|---|---|---|---|
| 13 | **密钥泄露** | API key、Postgres URL、MFA demo code 流到 git、日志行或大模型 prompt。 | (a) **`.env` 在 `.gitignore`**;seed 文件用的是 `'$argon2id$…'` 占位串。(b) **大模型 prompt 由固定字段白名单构造**(`accounts.balance_check`、`accounts.list` 等)——用户密码、OTP、完整账号、MFA 码、`OPENAI_API_KEY` 都不会被插进 prompt。(c) **日志行在写之前过滤** OTP 形态的 6 位数、PAN 形态的 16–19 位数、`sk-…` API key。(d) **审计链**只存 `parameters_masked`,手机尾号与金额都按展示契约脱敏。 | `llm_http.rs`(白名单)、audit masker、`.gitignore` |
| 14 | **日志 / 审计里的 PII** | 大模型 prompt、完整账号、原始备注被持久化在 `llm_calls` 或 `audit_events` 里,被管理员看到。 | (a) **`audit_events.parameters_masked`** 在 insert 前脱敏已知 PII 字段;未脱敏的 parameters 对象永远不到数据库。(b) **`llm_calls` 保留的是脱敏后的 prompt**——原始用户消息中的 PII 模式(手机尾号、完整卡号)被替换为占位符。(c) **保留期**:demo 的审计表无界(R-11);加固路径是按月分区 + 冷归档。(d) 管理员端点返回脱敏版;唯一能看到原文的是审计防篡改测试(`verify_chain`),它只比对哈希,不看内容。 | `audit.rs::mask_parameters`、`llm_audit.rs`、§4 R-11 |
| 15 | **内部人 / 管理员滥用**——管理员改写历史,或看到其他用户的数据 | 带 DB 写权限的恶意运维编辑 `audit_events`;恶意管理员调 `/api/v1/admin/users` 外泄数据。 | (a) **`audit_events` 在代码层仅追加**——grep 测试 `! grep -RE 'UPDATE audit_events' src/` 在任何写者被加入时让 CI 失败;`audit::append_event` 是唯一的插入点。(b) **哈希链**检测事后篡改:`prev_hash || canonical_json → sha256 → hash`,`verify_chain` 走完整表。(c) **角色分离**在 `AuthUser`:`/api/v1/admin/*` 要求 `admin` 角色;客户角色的会话即使带合法 cookie 也得到 `403 forbidden_role`。(d) **数据库级防护**(回收 audit 表的 UPDATE / DELETE 权限、加触发器)是加固路径(R-12)。 | `audit.rs`、`routes/admin.rs`、§4 R-12 |

## 7.6 如何阅读本矩阵

- **防线**列再次指向**代码**。如果某函数不存在,该行就是
  一条加固路径——把它链接到 [§4](./04-known-risks.md) 里
  对应的 `R-*`。
- 两行(1、4、6、7、9、10、13)的状态是**今天已在代码中**——
  这是不可妥协的银行不变式。
- 六行(2、3、8 部分、11 部分、12、14 保留期、15 DB 级)是
  **部分上线**,残余风险在 §4 具名。提交时"我们交付设计"
  的态度是 OK 的;生产化路径是具体的。
- 矩阵也是**回归目标**:每行都有对应测试(大多数在
  `backendrust/tests/`,SQL 注入防护是 grep 测试,审计链
  防护是 `audit_chain_integrity`)。

**返回:** [概览 →](./README.md) · [下一页: §8 — (保留) →](#)
