---
id: arch-frontend-zh
title: "4 · 前端 (中文)"
sidebar_label: "4 · 前端 (中文)"
sidebar_position: 5
---

# 4. 前端

> English: [04-frontend.md](../04-frontend.md)

前端是一个 **Next.js 14 App Router** 项目,使用 React 18 和
Tailwind。每一个面向用户的字符串都是双语的(英文 + 简体中文),
语言切换器在整应用中都接好了,不只是外壳。

## 4.1 高层布局

```
frontend/
├── src/
│   ├── app/                       App Router(基于文件系统的路由)
│   │   ├── layout.tsx             根 layout,providers,字体
│   │   ├── page.tsx               落地页(hero, CTAs)
│   │   ├── login/page.tsx         登录 / mock-MFA
│   │   ├── dashboard/page.tsx     Hero + 投资组合图 + KPIs
│   │   ├── accounts/page.tsx      账户列表 + 余额
│   │   ├── transfers/page.tsx     智能转账草稿 + 历史
│   │   ├── subscriptions/page.tsx 检测到的订阅
│   │   ├── budgets/page.tsx       按类目上限 + 超限
│   │   ├── wealth/page.tsx        财富产品 + 持仓
│   │   ├── forecast/page.tsx      "我负担得起吗?" 假设分析
│   │   ├── receipt/page.tsx       收据上传 + 草稿
│   │   ├── voice/page.tsx         语音备忘 → 草稿
│   │   ├── chat/page.tsx          Agent 聊天 + ConfirmationCard
│   │   ├── redteam/page.tsx       实时注入 / 反诈测试
│   │   ├── admin/                 管理面板
│   │   │   ├── page.tsx           /admin 索引
│   │   │   ├── overview/page.tsx  KPIs + 最近活动
│   │   │   ├── users/page.tsx     用户列表
│   │   │   ├── users/[id]/page.tsx 用户详情
│   │   │   ├── redteam/page.tsx   管理员视角的 red-team
│   │   │   ├── rules/page.tsx     Agent 规则编辑器
│   │   │   ├── audit/page.tsx     审计事件查看器
│   │   │   └── safety/page.tsx    安全事件查看器
│   │   ├── docs/page.tsx          应用内文档查看
│   │   ├── error.tsx              全局错误边界("SOMETHING BROKE")
│   │   ├── not-found.tsx          404
│   │   └── loading.tsx            Suspense 兜底
│   ├── components/
│   │   ├── layout/                app-shell, side-nav, top-bar
│   │   ├── admin/                 admin-shell, admin-nav
│   │   ├── motion/                aurora, fade-in, 等
│   │   ├── auth-provider.tsx      session + role
│   │   ├── confirmation-card.tsx  Yellow/Red 确认 UI
│   │   ├── language-button.tsx    EN / ZH 切换(消费方)
│   │   └── ui/                    按钮、对话框、select,…
│   ├── i18n/
│   │   ├── LanguageProvider.tsx   Context + 持久化 + SSR 安全水合
│   │   ├── dictionaries.ts        全部 EN / ZH 字符串
│   │   └── use-translate.ts       `t()` 钩子助手
│   ├── lib/
│   │   ├── api.ts                 fetch 包装(session + CSRF)
│   │   ├── store.ts               跨页面 bus + 小型 store
│   │   └── utils.ts               `cn`、格式化工具等
│   └── ...
├── public/                        静态资源
├── next.config.mjs
├── tailwind.config.ts
├── tsconfig.json
└── package.json
```

## 4.2 i18n — 三个组件

### 4.2.1 `LanguageProvider`

`frontend/src/i18n/LanguageProvider.tsx` 是一个 React context,
提供:

- `lang: Lang` — `"en" | "zh"`。
- `setLang(next, { persist? })` — 更新 state,可选写入
  `localStorage["ftp.lang"]`,把 `document.documentElement.lang`
  设为 `zh-CN` 或 `en`,若 `persist: true` 还把语言通过副作用
  发给后端。
- `t(key, vars?)` — 在当前字典中查 key,内插 `{var}` 占位符,
  缺失时回退到英文,再回退到 key 本身,这样拼写错误在 UI 上
  很明显。

SSR 安全:首次渲染总是返回 `"en"`。读取 `localStorage` 的
effect 仅在客户端运行,所以 SSR HTML 与首次客户端渲染一致,
**不会有水合不匹配**。

### 4.2.2 字典

`frontend/src/i18n/dictionaries.ts` 导出 `dictionaries` 对象,
按命名空间分键(例如 `hero.*`、`dashboard.*`、`transfer.*`、
`auth.*`、…)。每个命名空间在 `en` 和 `zh` 都有相同的 key。
类型 `DictKey` 是所有 key 的字面量联合,所以**缺 key 是
TypeScript 错误**。

| 命名空间 | 覆盖页面 |
|---|---|
| `common` | 按钮,"保存"、"取消"、"确认"、"重试" |
| `nav` | 侧边导航标签,面包屑分隔符 |
| `auth` | 登录表单、mock-MFA 文案、错误状态 |
| `hero` | 落地页 hero + CTAs |
| `dashboard` | Dashboard hero、KPI 条、面板标题 |
| `accounts` | 账户列表、卡号掩码展示 |
| `transfer` | 智能转账表单、安全审查卡 |
| `subscriptions` | 订阅列表、取消确认 |
| `budgets` | 预算行、超限横幅、"设置上限"表单 |
| `wealth` | 财富产品卡、适当性警告 |
| `forecast` | 预测图标签、verdict |
| `receipt` | 上传 UI、草稿复核、确认 |
| `chat` | 聊天输入、ConfirmationCard、"重试" |
| `admin` | 管理面板文案(规则、审计、安全) |
| `redteam` | Red-team 页文案(英文 + 示例攻击) |
| `voice` | 语音备忘 UI、转写状态 |
| `error` | 全局错误边界("SOMETHING BROKE" + 重试/返回) |

### 4.2.3 `useLanguage()` / `useT()` / `t()`

`useLanguage()` 返回 `{ lang, setLang, t }`。`useT()` 是个
语法糖,只返回 `t`,供不需要切换语言的组件使用。两个钩子
在 Provider 之外被调用时会抛错,这样 bug 在调用点就被抓住,
而不是在 render 时静默。

## 4.3 状态管理

没有 Redux、Zustand 或 Jotai。状态分布在三处:

1. **服务端数据** — 每个页面都是 `useEffect` + `fetch`,在
   mount 时重跑,并用 `Promise.all().catch(() => {})` 让一个
   端点失败不会让整个页面崩溃。dashboard 的 hero 与图表
   从 `/api/v1/accounts`、`/api/v1/reports/optimization`、
   `/api/v1/budgets` 等拉数据。
2. **跨页面 bus** — `frontend/src/lib/store.ts` 导出一个
   小型事件发射器加上几个类型化钩子:
   - `useAccountsBusTick()` — 当账户变更(例如其它地方确认了
     一笔转账)时,dashboard 重新拉取。
   - `useWealthBusTick()` — 当新增了持仓,wealth 页重新拉取。
   Bus 使用 `useSyncExternalStore` 以避免撕裂更新。
3. **表单状态** — 每个页面用本地 `useState`;较大表单(转账
   草稿、预算上限)在合适处用 `react-hook-form` 风格的
   `register + handleSubmit`,但多数是手写 `useState`,因为
   字段数很少。

## 4.4 API 包装器

`frontend/src/lib/api.ts` 导出 `request<T>(path, init)` 助手:

1. 自动读取 session cookie。
2. 把 CSRF token 从 `csrfToken` cookie(或 `meta[name=csrf-token]`,
   若 cookie 未设置)复制到 `X-CSRF-Token` header。
3. 为非 `GET` 请求设置 `Content-Type: application/json`。
4. 收到 401 时,清空 session 并跳转到 `/login`。
5. 抛出 `ApiError`,带上服务端的 `code` 与 `message`,这样
   页面可以渲染错误 toast。

CSRF 是 double-submit:服务端设置 `csrfToken` cookie,客户端把
它复制到 header。靠浏览器的同源策略 — 跨站请求读不到 cookie
值,也就填不了 header。

## 4.5 组件模式

| 模式 | 文件 | 说明 |
|---|---|---|
| `AppShell` | `components/layout/app-shell.tsx` | 包装除 `/login` 外的每个页面。包括侧栏、顶栏、语言切换、登出。 |
| `AdminShell` | `components/admin/admin-shell.tsx` | 包装 `/admin/*`。加 admin 专用导航与"返回应用"按钮。 |
| `ConfirmationCard` | `components/confirmation-card.tsx` | Yellow/Red 卡片。读 `risk_tier` 并渲染对应 CTA。`parameter_hash` 不匹配会被拒绝并出 toast。 |
| `FadeIn` | `components/motion/*` | 入场动画的轻量包装;尊重 `prefers-reduced-motion`。 |
| `Aurora` (PortfolioLine) | `components/motion/aurora.tsx` | Dashboard 上的 12 个月 SVG 折线图。接收 `months` prop,演示可在"年初至今"与"近 12 个月"之间切换。 |
| `Select` | `components/ui/select.tsx` | 类 Radix 的 select,第一个消费方就是语言切换器。 |

## 4.6 错误边界

`frontend/src/app/error.tsx` 是 Next.js App Router 的**全局
错误边界**。它捕获整个 app 在 render 时的错误,并渲染
"SOMETHING BROKE" 卡片,带三个 CTA:重试、返回、跳到 dashboard。
错误的 `digest` 会打印到控制台,这样 dev 可以在 Next dev 浮层
里 grep。

> 近期 dashboard 的 "SOMETHING BROKE" 回归就是由
> `frontend/src/app/dashboard/page.tsx` 中缺失的 `useLanguage`
> 导入触发的。boundary 捕获了 `ReferenceError`,dev 看到了卡片,
> 修复就是一行 import。**这就是 boundary 在起作用。**

## 4.7 路由 → API 对照表

| 路由 | 调用的后端端点 |
|---|---|
| `/login` | `POST /api/v1/auth/login`, `POST /api/v1/auth/mfa/verify` |
| `/dashboard` | `GET /api/v1/accounts`, `GET /api/v1/reports/optimization`, `GET /api/v1/budgets`, `GET /api/v1/subscriptions`, `GET /api/v1/safety/events?limit=5` |
| `/accounts` | `GET /api/v1/accounts`, `GET /api/v1/transactions?account_id=…` |
| `/transfers` | `GET /api/v1/payees`, `POST /api/v1/smart-transfers/parse`, `POST /api/v1/smart-transfers/security-review`, `POST /api/v1/transfers` |
| `/subscriptions` | `GET /api/v1/subscriptions`, `POST /api/v1/subscriptions/detect`, `POST /api/v1/subscriptions/{id}/cancel` |
| `/budgets` | `GET /api/v1/budgets`, `POST /api/v1/budgets`, `POST /api/v1/budgets/{id}/reserve` |
| `/wealth` | `GET /api/v1/wealth/products`, `GET /api/v1/wealth/holdings`, `POST /api/v1/wealth/subscribe` |
| `/forecast` | `POST /api/v1/forecast` |
| `/receipt` | `POST /api/v1/receipt/scan` |
| `/voice` | `POST /api/v1/voice/transcribe` |
| `/chat` | `POST /api/v1/agent/message`, `GET /api/v1/agent/conversations`, `POST /api/v1/actions/{id}/confirm`, `POST /api/v1/actions/{id}/verify` |
| `/redteam` | `POST /api/v1/redteam/test`, `GET /api/v1/redteam/samples` |
| `/admin/*` | `GET /api/v1/admin/overview`, `GET /api/v1/admin/users`, `GET /api/v1/admin/rules`, `POST /api/v1/admin/rules`, `GET /api/v1/admin/audit`, `GET /api/v1/admin/safety` |

## 4.8 构建与开发

- `next dev`(端口 3000)— HMR;保存时重新编译。
- `next build && next start` — 生产构建,单一产物。
- TypeScript strict 模式开启;提交代码里没有 `any`。
- ESLint 通过 `lint-staged` 在 `pre-commit` 阶段运行。
