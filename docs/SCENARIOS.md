# Scenarios

> All six scenarios ship in v1. Core four (1–4) are the demo spine; 5–6 are stretch gated on Phases 1–4 being green. Every screen says **"Simulation — no real money is moved."**

---

## 1. Smart transfers

### 1.1 Capabilities

| Feature | Example request | Behaviour | Tier |
|---|---|---|---|
| Transfer by name | "Pay Li Wei ¥200 for lunch" | Resolve payee; if ambiguous show candidates; show masked destination + exact amount | Yellow ≤ ¥1,000; Red above |
| Transfer by phone tail | "Send ¥100 to the contact ending 1234" | Match only verified mock contacts; clarify if > 1 match | Yellow / Red |
| Usual amount | "Send Li Wei the usual amount" | Read prior history; propose inferred amount; **disclose inference**; require confirm | Yellow |
| Scheduled transfer | "Pay rent on the first of each month" | Ask for amount + recipient if missing; show schedule; re-check policy at every run | Yellow / Red |
| Split bill | "Split ¥360 with Wang Fei and Anna" | Calculate shares; show allocation; create payment requests | Yellow |
| Transfer history | "When did I last pay Li Wei?" | Query ledger; return exact date + amount | Green |
| Transfer status | "Did the payment go through?" | Read operation status from backend; never infer success from submit | Green |

### 1.2 Safeguards

- Daily total tracked per user across **all** channels, server-side, `SELECT … FOR UPDATE`.
- Multiple small transfers cannot bypass the daily threshold.
- New recipients or changed destination escalate tier.
- Payee names, memos, merchant names are untrusted data, not instructions.
- Idempotency keys prevent double-execution on retries.
- Confirmation shows recipient, masked destination, amount, and effect.
- Scheduled transfers are re-authorised at execution; a past confirmation does not bypass current policy.
- If rollback is not available, the UI says so and offers an escalation path.

### 1.3 Mixed-language example

```
User: 转 200 给 Li Wei，午饭钱
Agent (parsed):
  intent = transfer
  entities.recipient_query = "Li Wei"
  entities.amount_fen = 20000
  entities.memo = "午饭钱"
  language = mixed
Plan:
  node[0] = transfer.create_pending { payee: payee_01H…, amount_fen: 20000, memo: "午饭钱" }
  tier = Yellow (within daily limit, verified payee)
UI: confirmation card → confirm → executor runs → audit event
```

---

## 2. Bill analysis

### 2.1 Capabilities

| Feature | Example | Behaviour | Tier |
|---|---|---|---|
| Spending by category | "Where did my money go in September?" | Totals + percentage shares + date range + chart data | Green |
| Monthly report | "Summarise this month" | Income, expenses, category totals, MoM comparison when available | Green |
| Annual report | "Make my 2026 spending report" | Aggregate; flag missing coverage | Green |
| Anomaly candidates | "Anything strange this month?" | Rule-based flags (see §2.2) | Green |
| Explain a flag | "Why is the ¥899 charge unusual?" | Show amount, baseline, merchant history, reasons | Green |
| Export report | "Export my monthly report" | CSV / PDF from backend-derived data | Green (with minimisation) |

### 2.2 Anomaly rules (initial)

A transaction is "potentially unusual" if **any** rule hits (each rule produces an evidence string):

- Amount is much higher than the user's historical median for that category (e.g. > 3×).
- Merchant appears for the first time.
- Two similar amounts at the same merchant within 7 days.
- Charge at an unusual hour (if timestamp available).
- Recurring merchant's price increased (e.g. > 10% over the prior 3 cycles).
- Category or merchant differs from the user's typical pattern for that merchant.

A statistical ranking (IQR or z-score) is layered on top when enough history exists. If not enough data, the flag is rule-based only and the UI says so.

**Never** label a flagged transaction "fraud" without sufficient evidence. The UI says "potentially unusual" with reasons.

### 2.3 Determinism

Totals come from `reports/spending` SQL. The LLM only narrates and may not alter the figures. A unit test seeds a known ledger and asserts the report bytes match.

---

## 3. Subscriptions and direct debit

### 3.1 Capabilities

| Feature | Example | Behaviour | Tier |
|---|---|---|---|
| Discover | "What am I subscribed to?" | Identify merchants charged at regular intervals with similar amounts | Green |
| Renewal reminder | "Remind me three days before renewal" | Create reminder based on estimated next charge | Green |
| Price-change warning | "Did any subscription become more expensive?" | Compare recurring amounts; show diff | Green |
| Cancel | "Cancel the video app" | Resolve merchant, show amount and expected next charge, request confirm, then mock merchant connector | Yellow |
| Direct-debit review | "Who can debit my account?" | List active mandates, last charge, amount | Green |
| Verify cancellation | "Was it cancelled?" | Read connector's status; show pending/confirmed/failed accurately | Green |

### 3.2 Cancellation boundary

For the demo, cancellation updates a **simulated** subscription record. A real transaction history alone cannot prove that a subscription can be cancelled, so we route through a mock merchant connector. The UI is explicit about this boundary.

### 3.3 Detection

Rules: same merchant appears ≥ 3 times, amounts within ±10% of median, intervals within ±5 days of the median interval. Each subscription's `evidence` field stores the rule hits and the transaction IDs that contributed.

---

## 4. Cross-scenario orchestration (showcase)

### 4.1 Example request

> "My wife's birthday is on 20 November. Reserve ¥1,000 for it and order flowers and a cake two days before."

### 4.2 Plan

| Step | Tool | Depends on | Tier | Notes |
|---|---|---|---|---|
| 0 | `profile.resolve_date` | — | Green | Resolve the date; clarify if year ambiguous |
| 1 | `accounts.balance_check` | 0 | Green | Confirm available balance ≥ ¥1,000 + order total |
| 2 | `goals.create_reservation` | 1 | Yellow | Hold ¥1,000 earmark |
| 3 | `connectors.merchant.order` (flowers) | 0 | Yellow | Order for 18 Nov |
| 4 | `connectors.merchant.order` (cake) | 0 | Yellow | Order for 18 Nov |
| 5 | `reminders.create` | 0 | Green | Reminder on 19 Nov |
| 6 | `goals.consume_reservation` (or release) | 2 | Yellow | On the day or after, depending on outcome |

If any step fails, the workflow **pauses**. Compensation runs in reverse order where a safe compensating action exists (release the reservation, cancel the mock orders). The user is shown which steps are reversible and which are not.

### 4.3 Demo angle

The DAG visualises dependencies. A judge can see:

- The plan before any money moves.
- Yellow confirmations per money-moving step.
- Pause + compensate in action.
- A clear "this step cannot be reversed" callout for any external order that already left the sim.

---

## 5. Wealth operations (stretch)

### 5.1 Read

- `GET /wealth/products` lists the catalog.
- `GET /wealth/products/{id}` shows risk level, expected return, minimum subscription, and suitability rules.

### 5.2 Suitability

Enforced server-side. See `POLICY.md §5`. The LLM never decides suitability.

### 5.3 Simulated fulfilment

- `POST /wealth/subscribe` (Yellow or Red depending on risk) creates a `wealth_holdings` row and writes a corresponding `transactions` row tagged `category=wealth`. No external API is called.
- `POST /wealth/redeem` (Yellow) reverses the holding and posts a credit transaction.
- The UI clearly labels every screen as **simulation**.

### 5.4 Out of scope

Real investment recommendations, real product subscriptions, suitability engine beyond the seed rules.

---

## 6. Card management (stretch)

### 6.1 Full virtual-card lifecycle in simulation

| Action | Tier | Result |
|---|---|---|
| Issue card | Yellow | New row in `cards`, masked number, default limits |
| Freeze | Yellow | `status=frozen`; simulator refuses to "swipe" |
| Unfreeze | Yellow | `status=active` |
| Report lost | Red | `status=lost`; immediate cancel + reissue flow |
| Cancel | Yellow | `status=cancelled` |
| Update limits | Yellow | Update `single_tx_limit_fen` / `monthly_limit_fen` |

### 6.2 Why this matters

Cards are the highest-friction real-world case (lost, stolen, fraud). The lifecycle exercises: status transitions, parameter hashing on limit changes, Red tier on report-lost (MFA required), and audit completeness.

### 6.3 Out of scope

Real card network integration, real KYC, real BIN ranges. Numbers are simulator-only and never look like real card BINs.

---

## 7. Out-of-scope for v1

- Real bank accounts, real payment rails, real money movement.
- Real identity documents or banking credentials.
- Real investment recommendations.
- Production regulated deployment.
- Voice / image banking until text workflow is reliable.
