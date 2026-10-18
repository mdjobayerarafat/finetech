---
id: arch-algorithms
title: "3 · Core Algorithms"
sidebar_label: "3 · Core Algorithms"
sidebar_position: 4
---

# 3. Core Algorithms

> 中文: [03-核心算法 (中文)](./zh/03-algorithms.md)

FinTech Pioneers's value is in **seven** specific algorithms. This page
documents each one in the same format: **intent**, **pseudocode**,
**source map**, **complexity / guarantees**, **testability hook**.

> **The non-negotiable rule** that all of these share: *code
> computes, the model explains.* The LLM only ever narrates a
> number that the backend already produced. There is no path in
> the codebase where the LLM's answer is taken at face value as
> a financial figure.

---

## 3.1 Policy tier decision — `policy::decide_*`

**Intent.** Given a validated tool call, decide whether it is
`Green`, `Yellow`, or `Red` and emit a list of human-readable
flags. The decision is **monotonic in one workflow**: escalate
is allowed, downgrade is not.

**Source.** `backendrust/src/policy.rs`. The per-tool decision
functions are: `decide_transfer`, `decide_card_action`,
`decide_wealth_subscribe`, `decide_wealth_redeem`,
`decide_subscription_cancel`, `decide_goal_reservation`,
`decide_budget_set`, `decide_budget_reserve`.

**Algorithm — `decide_transfer`.**

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

**Guarantees.**

- **Monotone in a workflow** — there is no `tier: downgraded`
  path. Escalation is one-way.
- **No integer / float drift** — `amount_fen` is `i64`; thresholds
  are constants in `config::Settings`.
- **Memo is sanitised first** — the injection check happens before
  the LLM ever sees the value, and the LLM is never given raw
  user-supplied free-form text as an instruction.

**Testability.** `backendrust/src/policy.rs` is exhaustively unit
tested in `policy::tests` (compile-time `#[cfg(test)]` block).
Coverage includes: zero amount, negative amount, daily limit
exceeded, payee escalation flags, memo injection phrases, scam
guard hard signals.

---

## 3.2 Scam / fraud guard — `fraud::score_transfer`

**Intent.** Produce a list of *signals* (not a tier) so the policy
engine and the chat UI can show "why does the bank think this
might be a scam?" The LLM is asked to narrate the signal list;
the score itself is computed by Rust.

**Source.** `backendrust/src/fraud.rs`. Key functions:
`score_transfer`, `narrate_scam_flags`, `detect_urgency`,
`detect_flagged_merchant`, `is_round_number`,
`historical_median_fen`, `average_monthly_expense_fen`,
`available_balance_fen`.

**Algorithm.**

```
function score_transfer(user, payee, amount_fen, memo):
    flags = []

    if payee.first_used_at is None:
        flags += [FraudFlag("new_payee", weight=3)]

    if not payee.verified:
        flags += [FraudFlag("unverified_payee", weight=4)]

    median = historical_median_fen(payee.id)        # last 3+ transfers
    if median > 0 and amount_fen > 3 * median:
        flags += [FraudFlag("amount_unusual", weight=4)]

    if detect_urgency(memo) ≠ ∅:                     # bilingual list, 35+ phrases
        flags += [FraudFlag("urgency_words", weight=5)]

    if detect_flagged_merchant(payee.name) ≠ ∅:      # "stranger", "高回报", …
        flags += [FraudFlag("payee_flagged", weight=5)]

    if is_round_number(amount_fen) and amount_fen ≥ 50_000:
        flags += [FraudFlag("amount_round", weight=1)]

    avg_expense = average_monthly_expense_fen(user.id)   # 90-day debit avg / 3
    available  = available_balance_fen(user.id)          # sum of account balances
    if available - amount_fen < avg_expense:
        flags += [FraudFlag("out_of_pattern", weight=3)]

    score = min(100, sum(weights) * 4)                # gauge for the demo UI
    templated = "This transfer has N signal(s): …" if flags else "No scam indicators."
    return FraudAssessment(flags, score, templated, narrative=None)
```

**Hard-signal rule.** A transfer is escalated to **Red** if any of:

- `urgency_words` is present, **or**
- `payee_flagged` is present, **or**
- any flag has `weight ≥ 5`.

This is enforced by the policy engine reading the assessment, not
by the fraud guard itself — `score_transfer` is **advice** to the
policy engine; it never returns a tier.

**Narrative.** `narrate_scam_flags` calls the LLM only when
`UNSLOTH_ENABLED=1`. The prompt is a short JSON of signals; the
LLM is asked to write 1–2 sentences. The response is **not** used
for any decision; the score stays the same whether the narrative
is present or not. If the LLM fails, the templated explanation is
returned as a fallback so the UI is never empty.

**Static seed lists.** `URGENCY_PHRASES` is bilingual (35+ English
phrases plus `马上`, `立刻`, `尽快`, `保密`, `税务局`, `公安局`, …).
`FLAGGED_MERCHANT_KEYWORDS` includes `stranger`, `unknown person`,
`doubling`, `high return`, `easy money`, `陌生人`, `高回报`,
`投资机会`, `翻倍`. In a real bank these come from a fraud feed;
here they are static and clearly labelled simulation-only.

---

## 3.3 Smart-transfer parse — `smart_transfers::parse_request`

**Intent.** Turn a free-form transfer request ("转 200 元午饭钱给
Li Wei, tomorrow, split with Alice") into the same JSON shape the
planner expects: `amount_fen`, `recipient_query`, `memo`,
`schedule_iso`, `split_participants`, `needs_clarification`,
`questions[]`.

**Source.** `backendrust/src/smart_transfers.rs`. Entry points:
`parse_request`, `suggest_payees`, `resolve`.

**Algorithm — LLM path (when `UNSLOTH_ENABLED=1`).**

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
    resolved = resolve(user, parsed.recipient_query)   # always runs server-side
    return parsed + {
        resolved_payee: resolved.payee,
        resolve_status:  resolved.status,         # resolved | ambiguous | none
        candidates:      resolved.candidates,
    }
```

**Algorithm — stub path (offline / LLM unavailable).**

```
function stub_parse(text):
    amount_fen = regex_amount(text)               # (\d+)([.,]\d{1,2})? (元|块|RMB|¥|￥|yuan)?
    recipient_query = first_token_after(["pay ","to ","send to ","给","转给"])
                       OR longest CJK token
                       OR first capitalised ASCII token
    schedule_iso = if "tomorrow" / "明天" / etc → now + 1d
                   if "next week" / "下周"     → now + 7d
                   if "next month" / "下月"    → now + 30d
                   else None
    wants_split  = "split" ∈ lower OR "均摊" ∈ text OR "分摊" ∈ text
    memo         = substring after " for " / "memo:" / "因为" / "——", trimmed to 200 chars
    return { amount_fen, recipient_query, memo, schedule_iso,
             split_participants, needs_clarification, questions[] }
```

**Resolve step** (shared by both paths).

```
function resolve(user, query):
    suggestions = suggest_payees(user, query, limit=5)   # DB query + scoring
    if suggestions is empty:
        return ResolveResult(status="none",
            note="No saved payee matches. Add a beneficial account first.")
    if len == 1 or top.confidence ≥ 0.95:
        return ResolveResult(status="resolved", payee=suggestions[0].payee)
    return ResolveResult(status="ambiguous", candidates=suggestions)
```

`PayeeSuggestion` carries a `match_kind` (`exact_name`,
`phone_tail`, `prefix_name`, `contains_name`) and a `confidence`
(1.0, 0.98, 0.85, 0.6).

**Security review.** `security_review` is a separate helper called
on the *draft* transfer. It runs `fraud::score_transfer` plus the
injection and urgency detectors, then returns a `SecurityReview`
struct with `recommended_tier`, `fraud_score`, `flags`,
`injection_phrase`, `urgency_detected`, `summary` (LLM or
templated), and `schedule_note`. **The recommended tier is still
advisory** — the actual tier decision happens in
`policy::decide_transfer` when the user confirms.

---

## 3.4 "Can I afford it?" — `forecast::run`

**Intent.** Given a scenario (spend `amount_fen` in `months`
months), project the balance forward for `months` months using
rolling 90-day averages plus any scheduled reminders, and return
a verdict (`yes` / `tight` / `no`) and a narrative.

**Source.** `backendrust/src/forecast.rs`. Entry: `run(pool, settings, user, amount_fen, spend_month_index, months)`.

**Algorithm.**

```
function run(user, amount_fen, spend_month_index, months):
    months = clamp(months, 1, 24)
    spend_month_index = clamp(spend_month_index, 0, months - 1)

    starting      = available_balance_fen(user.id)
    avg_income    = monthly_cashflow_fen(user.id).income   # 90-day credit sum / 3
    avg_expense   = monthly_cashflow_fen(user.id).expense  # 90-day debit  sum / 3
    monthly_subs  = monthly_subscriptions_fen(user.id)     # Σ(amt × 30 / interval_days) for active
    held          = held_reservations_fen(user.id)         # reservations with state='held'
    budgets       = active_budgets(user.id)                # per-category limit / 30-day spend / breached

    upcoming = upcoming_bills_in_window(user.id, from=first_of_next_month, to=+months)

    effective_starting = starting - held                  # committed cash deflates the baseline
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

**`build_series` is the per-month loop.** For each month `i`:

```
bal += (avg_income - avg_expense) - sum(bills due this month)
baseline_bal += (avg_income - avg_expense) - sum(bills due this month)
if i == spend_month_index and amount_fen > 0:
    bal -= amount_fen
push SeriesPoint(month_index=i, label="Jan 2026", …)
```

> **Note.** Subscriptions are *not* subtracted twice. They are
> already inside `avg_expense` because they appear in the
> transaction history. The forecast surfaces `monthly_subs` as a
> field but does not double-count.

**`verdict`.**

```
if end_balance < 0:                       return "no"
elif end_balance < max(amount_fen/2, 50000): return "tight"
else:                                       return "yes"
```

**Narrative.** LLM only if `UNSLOTH_ENABLED=1`. The system prompt
is explicit: "Numbers must match exactly; do not invent any."

---

## 3.5 Receipt scan — `receipt::scan`

**Intent.** Upload a photo of a receipt/invoice and produce a
**draft** transfer (`payee_hint`, `amount_fen`, `due_at`,
`proposed_arguments`) that the user confirms with one tap.

**Source.** `backendrust/src/receipt.rs`. Entry: `scan(state, auth, rid, filename, declared_hint, declared_amount, bytes_len)`.

**Algorithm.**

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

**Stub determinism.** `stub_extract` uses
`(filename.len() XOR declared_hint.len()) % len(DEMO_RECEIPTS)` to
pick one of 5 deterministic demo receipts (Sun Cafe latte,
Acme Books invoice, Cell Co. telco, CityPower electric, University
tuition). The result is reproducible for the same input — important
for the demo.

**Audit hygiene.** Only `filename[:64]`, `bytes`, `source`, and
`confidence` are logged. The extracted payee and amount are
**not** in `parameters_masked` (the user can re-derive them by
viewing the draft).

**Hard guarantee.** A scanned receipt is **always** a draft. It
never executes. The only way to post a transfer is through
`transfer.create_pending` after the user taps confirm.

---

## 3.6 Planner DAG — `planner::build_plan` + `decide_node`

**Intent.** Build a workflow of one or more tool nodes from a
validated `Proposal`. The planner is the **only** code that calls
`policy::decide_*`, `tool_spec::validate_arguments`, and the
agent rules engine.

**Source.** `backendrust/src/planner.rs`. The static tool
registry lives in `tool_spec.rs::SPECS` (Lazy-initialised
`Vec<ToolSpec>`); each entry has a `canonical_name`,
`mutating`, `default_tier`, `arguments_schema`, and a list of
required/optional `SlotSpec`s.

**Algorithm.**

```
function build_plan(user, proposal):
    plan = []
    for action in proposal.proposed_actions:
        tool = canonicalize_tool_name(action.tool)
        spec = tool_spec::spec_for(tool)
        if not spec: continue                          # unknown tool → skip silently
        if tool is in forbidden_tools():               # admin rules
            return PlanError(Forbidden, audit("agent_rule.match"))
        validate_arguments(spec, action.arguments)      # schema + slot check
        slot_bindings = collect slot_bindings from conversation memory
        for slot in spec.required_slots:
            if slot_bindings[slot.name] is empty:
                if slot.inferable: try retriever / wallet view
                else:               return PlanError(Clarification, slot.question)

        node = decide_node(user, spec, action.arguments)
        plan.append(node)
    return plan

function decide_node(user, spec, args):
    # 1) tool-specific decision function (if any)
    decision = call decide_<tool>(args) or Decision(default_tier=spec.default_tier)

    # 2) admin rules
    decision = apply_agent_rules(decision, user, args)  # can escalate, never downgrade

    # 3) parameter_hash + (for Yellow/Red) pending_action row
    if decision.tier in (YELLOW, RED):
        parameter_hash = canonical::parameter_hash(args)
        pending_id = insert pending_actions(
            tool=spec.canonical_name, arguments=args, risk_tier=tier,
            policy_reason=decision.reason, parameter_hash, expires_at=now+5m
        )
        return PlannedNode(risk_tier=tier, pending_action_id=pending_id, parameter_hash)

    # 4) Green → execute inline
    return PlannedNode(risk_tier=GREEN)                   # no pending row
```

**Compensation.** Every node declares a `compensation` string
(e.g. `"reverse_transfer"`). If a downstream node fails, the
executor walks the DAG in reverse and runs the compensation
function for each completed node. This is what makes the workflow
"all-or-nothing" without requiring the user to re-confirm.

**Idempotency.** The `idempotency_key` for a tool is
`workflow_id + node_id + parameter_hash`. Replays are no-ops —
`INSERT … ON CONFLICT (idempotency_key) DO NOTHING` in the
underlying SQL.

---

## 3.7 Subscription detection — heuristic (admin page only)

**Intent.** Surface recurring charges in the user's transaction
history so the agent can prompt "would you like to cancel X?".
The detection runs in a server-side route
(`routes/subscriptions.rs::detect`), not in real time on the chat
path.

**Algorithm — sliding window.**

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

**Heuristic limitations (called out in code comments).**

- The seed dataset has irregular intervals — the seed marks
  the obvious ones as `active` and leaves the rest.
- Annual subscriptions are easy to miss with 180 days of history.
- A merchant with one large and many small charges gets bucketed
  by the small ones.
- In production this would be a real recurring-billing detection
  pipeline (Stripe Sigma, Plaid Recurring, internal). The
  simulation comments say so explicitly.

---

## 3.8 Tool loop termination

The LLM is invoked **once per chat turn** (`llm::propose`). The
planner turns its `proposed_actions[]` into a DAG. There is no
iterative "call LLM → see tool result → call LLM again" loop in
production. The reason: every LLM call adds latency and a chance
for prompt injection. The stub proposer is regex-based and
deterministic, so the loop is fast and safe.

The single-turn structure is what makes `parameters_masked`
sufficient: the LLM never sees the post-tool state directly.
The user's next turn is a fresh `propose` call with a
`state_summary` rebuilt by `retriever.rs` from the audit log and
conversation memory.

---

## 3.9 Algorithm summary table

| # | Algorithm | Input | Output | Source | LLM? |
|---|---|---|---|---|---|
| 3.1 | Policy tier decision | (user, account, payee, amount, memo) | `Decision` (tier + flags) | `policy.rs::decide_*` | No |
| 3.2 | Fraud guard | (user, payee, amount, memo) | `FraudAssessment` (flags + score + narrative) | `fraud.rs` | Narrative only |
| 3.3 | Smart-transfer parse | (user, free text) | `{amount_fen, recipient, memo, schedule, …}` | `smart_transfers.rs` | Optional |
| 3.4 | Forecast | (user, amount, month, months) | `ForecastOut` (series + verdict + narrative) | `forecast.rs` | Narrative only |
| 3.5 | Receipt scan | (file metadata, hints) | `ReceiptDraft` | `receipt.rs` | Optional |
| 3.6 | Planner DAG | `Proposal` | `PlannedNode[]` with tier + pending rows | `planner.rs` + `tool_spec.rs` | No |
| 3.7 | Subscription detection | (user, lookback) | subscriptions upserted | `routes/subscriptions.rs` | No |
| 3.8 | LLM tool loop | chat turn | (single-shot propose) | `llm.rs` | Yes (with stub) |
