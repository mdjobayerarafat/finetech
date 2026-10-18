---
id: arch-frontend
title: "4 · Frontend"
sidebar_label: "4 · Frontend"
sidebar_position: 5
---

# 4. Frontend

> 中文: [04-前端 (中文)](./zh/04-frontend.md)

The frontend is a **Next.js 14 App Router** project with React 18
and Tailwind. Every user-facing string is bilingual (English +
Simplified Chinese) and the language toggle is wired through the
app, not just the chrome.

## 4.1 High-level layout

```
frontend/
├── src/
│   ├── app/                       App Router (file-system routing)
│   │   ├── layout.tsx             Root layout, providers, fonts
│   │   ├── page.tsx               Landing page (hero, CTAs)
│   │   ├── login/page.tsx         Sign-in / mock-MFA
│   │   ├── dashboard/page.tsx     Hero + portfolio chart + KPIs
│   │   ├── accounts/page.tsx      Account list + balances
│   │   ├── transfers/page.tsx     Smart-transfer drafts + history
│   │   ├── subscriptions/page.tsx Detected subscriptions
│   │   ├── budgets/page.tsx       Per-category caps + breach
│   │   ├── wealth/page.tsx        Wealth product list + holdings
│   │   ├── forecast/page.tsx      "Can I afford it?" what-if
│   │   ├── receipt/page.tsx       Receipt upload + draft
│   │   ├── voice/page.tsx         Voice-memo → draft
│   │   ├── chat/page.tsx          Agent chat + ConfirmationCard
│   │   ├── redteam/page.tsx       Live injection / scam tester
│   │   ├── admin/                 Admin panel
│   │   │   ├── page.tsx           /admin index
│   │   │   ├── overview/page.tsx  KPIs + recent activity
│   │   │   ├── users/page.tsx     User list
│   │   │   ├── users/[id]/page.tsx User detail
│   │   │   ├── redteam/page.tsx   Admin view of red-team panel
│   │   │   ├── rules/page.tsx     Agent rules editor
│   │   │   ├── audit/page.tsx     Audit-event inspector
│   │   │   └── safety/page.tsx    Safety-event inspector
│   │   ├── docs/page.tsx          In-app docs viewer
│   │   ├── error.tsx              Global error boundary ("SOMETHING BROKE")
│   │   ├── not-found.tsx          404
│   │   └── loading.tsx            Suspense fallback
│   ├── components/
│   │   ├── layout/                app-shell, side-nav, top-bar
│   │   ├── admin/                 admin-shell, admin-nav
│   │   ├── motion/                aurora, fade-in, etc.
│   │   ├── auth-provider.tsx      session + role
│   │   ├── confirmation-card.tsx  Yellow/Red confirmation UI
│   │   ├── language-button.tsx    EN / ZH toggle (consumer)
│   │   └── ui/                    buttons, dialogs, select, …
│   ├── i18n/
│   │   ├── LanguageProvider.tsx   Context + persist + SSR-safe hydrate
│   │   ├── dictionaries.ts        All EN / ZH strings
│   │   └── use-translate.ts       `t()` hook helper
│   ├── lib/
│   │   ├── api.ts                 fetch wrapper (session + CSRF)
│   │   ├── store.ts               cross-page bus + small store
│   │   └── utils.ts               `cn`, formatters, etc.
│   └── ...
├── public/                        static assets
├── next.config.mjs
├── tailwind.config.ts
├── tsconfig.json
└── package.json
```

## 4.2 i18n — the three pieces

### 4.2.1 `LanguageProvider`

`frontend/src/i18n/LanguageProvider.tsx` is a React context with:

- `lang: Lang` — `"en" | "zh"`.
- `setLang(next, { persist? })` — updates state, optionally writes
  to `localStorage["ftp.lang"]`, sets `document.documentElement.lang`
  to `zh-CN` or `en`, and (if `persist: true`) sends the language
  to the backend via a side-effect.
- `t(key, vars?)` — looks up the key in the active dictionary,
  interpolates `{var}` placeholders, and falls back to English
  then to the key itself so a typo is obvious in the UI.

SSR safety: the first render always returns `"en"`. The effect
that reads `localStorage` runs only on the client, so the
server-rendered HTML matches the first client render and there's
no hydration mismatch.

### 4.2.2 Dictionary

`frontend/src/i18n/dictionaries.ts` exports a `dictionaries`
object keyed by namespace (e.g. `hero.*`, `dashboard.*`,
`transfer.*`, `auth.*`, …). Each namespace has the same keys
in both `en` and `zh`. The type `DictKey` is a union literal of
every key, so a missing key is a TypeScript error.

| Namespace | Pages it covers |
|---|---|
| `common` | Buttons, "Save", "Cancel", "Confirm", "Retry" |
| `nav` | Side-nav labels, breadcrumb separators |
| `auth` | Login form, mock-MFA copy, error states |
| `hero` | Landing-page hero + CTAs |
| `dashboard` | Dashboard hero, KPI strip, panel titles |
| `accounts` | Account list, masked-number display |
| `transfer` | Smart-transfer form, security-review card |
| `subscriptions` | Subscription list, cancel confirm |
| `budgets` | Budget rows, breach banner, "set a cap" form |
| `wealth` | Wealth product cards, suitability warnings |
| `forecast` | Forecast chart labels, verdicts |
| `receipt` | Upload UI, draft review, confirm |
| `chat` | Chat input, ConfirmationCard, "retry" |
| `admin` | Admin panel copy (rules, audit, safety) |
| `redteam` | Red-team page copy (English + sample attacks) |
| `voice` | Voice memo UI, transcription states |
| `error` | Global error boundary ("SOMETHING BROKE" + retry/go back) |

### 4.2.3 `useLanguage()` / `useT()` / `t()`

`useLanguage()` returns `{ lang, setLang, t }`. `useT()` is a
sugar that returns only `t` for components that don't need to
switch languages. Both hooks throw if used outside the provider
so the bug is caught at the call site, not silently at render.

## 4.3 State management

There is no Redux, no Zustand, no Jotai. State lives in three
places:

1. **Server data** — every page is a `useEffect` + `fetch` that
   re-runs on mount, with `Promise.all().catch(() => {})` so a
   failed endpoint doesn't crash the page. The dashboard's hero
   and chart pull from `/api/v1/accounts`,
   `/api/v1/reports/optimization`, `/api/v1/budgets`, etc.
2. **Cross-page bus** — `frontend/src/lib/store.ts` exports a
   tiny event-emitter plus a few typed hooks:
   - `useAccountsBusTick()` — the dashboard re-fetches when an
     account changes (e.g. a transfer confirmed elsewhere).
   - `useWealthBusTick()` — the wealth page re-fetches when a
     new holding is added.
   The bus uses `useSyncExternalStore` for tear-free updates.
3. **Form state** — local `useState` in each page; bigger forms
   (transfer draft, budget cap) use the `react-hook-form` style
   of `register + handleSubmit` where it helps, but mostly
   hand-rolled `useState` since the field count is small.

## 4.4 API wrapper

`frontend/src/lib/api.ts` exports a `request<T>(path, init)`
helper that:

1. Reads the session cookie automatically.
2. Sends the CSRF token from the `csrfToken` cookie (or the
   `meta[name=csrf-token]` if the cookie isn't set) as the
   `X-CSRF-Token` header.
3. Sets `Content-Type: application/json` for non-`GET` requests.
4. On 401, clears the session and redirects to `/login`.
5. Throws an `ApiError` with the server's `code` and `message`
   so the page can render an error toast.

CSRF is double-submit: the server sets a `csrfToken` cookie, the
client copies it into the header. The browser's same-origin
policy is what makes this work; cross-site requests can't read
the cookie value to set the header.

## 4.5 Component patterns

| Pattern | Files | Notes |
|---|---|---|
| `AppShell` | `components/layout/app-shell.tsx` | Wraps every page except `/login`. Includes side-nav, top bar, language toggle, sign-out. |
| `AdminShell` | `components/admin/admin-shell.tsx` | Wraps `/admin/*`. Adds admin-only nav and a back-to-app button. |
| `ConfirmationCard` | `components/confirmation-card.tsx` | The Yellow/Red card. Reads `risk_tier` and renders the right CTA. Mismatched `parameter_hash` is rejected with a toast. |
| `FadeIn` | `components/motion/*` | Tiny wrapper for entrance animations; respects `prefers-reduced-motion`. |
| `Aurora` (PortfolioLine) | `components/motion/aurora.tsx` | The 12-month SVG line chart on the dashboard. Accepts a `months` prop so the demo can switch "year to date" vs "trailing 12 months." |
| `Select` | `components/ui/select.tsx` | Radix-style select with the language switcher as its first consumer. |

## 4.6 Error boundary

`frontend/src/app/error.tsx` is a Next.js App Router **global
error boundary**. It catches render-time errors anywhere in the
app and renders the "SOMETHING BROKE" card with three CTAs:
retry, go back, jump to dashboard. The error's `digest` is
logged to the console so the dev can grep the Next dev overlay.

> The recent dashboard "SOMETHING BROKE" regression was caused
> by a `useLanguage` import missing in
> `frontend/src/app/dashboard/page.tsx`. The boundary caught the
> `ReferenceError`, the dev saw the card, and the fix was a
> one-line import change. **This is the boundary doing its job.**

## 4.7 Route → API map

| Route | Backend endpoints used |
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

## 4.8 Build and dev

- `next dev` (port 3000) — HMR; recompiles on save.
- `next build && next start` — production build, single output.
- TypeScript strict mode is on; no `any` in committed code.
- ESLint runs on `pre-commit` via a `lint-staged` config.
